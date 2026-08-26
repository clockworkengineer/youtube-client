use std::path::Path;
use google_youtube3::{YouTube, hyper_rustls, hyper_util};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};

pub mod config;
pub mod utils;

pub use config::*;

#[derive(thiserror::Error, Debug)]
pub enum YoutubeError {
    #[error("Credentials error: {0}")]
    Credentials(String),

    #[error("Authentication error: {0}")]
    Auth(#[from] yup_oauth2::Error),

    #[error("API request failed: {0}")]
    Api(#[from] google_youtube3::Error),

    #[error("API Quota exceeded. Please check your Google Developer Console quota limits: {0}")]
    QuotaExceeded(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Media error: {0}")]
    Media(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("yt-dlp is missing. Please make sure yt-dlp is installed and in your PATH: {0}")]
    YtDlpMissing(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, YoutubeError>;

pub const YOUTUBE_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/youtube",
    "https://www.googleapis.com/auth/youtube.force-ssl",
    "https://www.googleapis.com/auth/youtube.readonly",
];

async fn retry_api_call<F, Fut, T>(f: F) -> std::result::Result<T, YoutubeError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, google_youtube3::Error>>,
{
    use rand::Rng;
    let mut attempts = 0;
    let mut delay = std::time::Duration::from_millis(500);
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempts += 1;
                
                // Check if the error indicates a quota limit breach (commonly status code 403 or failure messages)
                if let google_youtube3::Error::Failure(ref resp) = e {
                    if resp.status().as_u16() == 403 {
                        // In google-youtube3/hyper client, we can't easily extract bytes from hyper::Response BoxBody in a simple way
                        // since BoxBody isn't easily readable without pinning and polling. Let's inspect the response headers or simply
                        // look at the string representation or status.
                        let status_str = format!("{:?}", resp);
                        if status_str.contains("quotaExceeded") || status_str.contains("Quota Exceeded") || status_str.contains("403") {
                            return Err(YoutubeError::QuotaExceeded(status_str));
                        }
                    }
                }

                if attempts >= 3 {
                    return Err(YoutubeError::Api(e));
                }
                let is_retryable = match &e {
                    google_youtube3::Error::HttpError(_) => true,
                    google_youtube3::Error::Failure(resp) => {
                        let status = resp.status();
                        status.is_server_error() || status.as_u16() == 429
                    }
                    _ => false,
                };
                if !is_retryable {
                    return Err(YoutubeError::Api(e));
                }
                
                // Exponential backoff with jitter: delay * (0.5 to 1.5)
                let jitter: f64 = rand::thread_rng().gen_range(0.5..1.5);
                let wait_duration = delay.mul_f64(jitter);
                
                tokio::time::sleep(wait_duration).await;
                delay *= 2;
            }
        }
    }
}

/// Initialize a `YoutubeClient` with resolved credentials and standard YouTube scopes.
pub async fn init_client(
    opt_client_id: Option<String>,
    opt_client_secret: Option<String>,
    config_path: &Path,
    token_cache_path: &Path,
) -> Result<YoutubeClient> {
    let (client_id, client_secret) = resolve_credentials(opt_client_id, opt_client_secret, config_path)?;
    
    let resolved_token_cache = if token_cache_path == Path::new("tokencache.json") && !token_cache_path.exists() {
        if let Some(global_dir) = crate::config::get_global_config_dir() {
            let _ = std::fs::create_dir_all(&global_dir);
            global_dir.join("tokencache.json")
        } else {
            token_cache_path.to_path_buf()
        }
    } else {
        token_cache_path.to_path_buf()
    };

    YoutubeClient::new_oauth_with_scopes(
        &client_id,
        &client_secret,
        &resolved_token_cache,
        YOUTUBE_SCOPES,
    )
    .await
}


fn extract_thumbnail_url(thumbnails: Option<google_youtube3::api::ThumbnailDetails>) -> String {
    if let Some(t) = thumbnails {
        if let Some(url) = t.high.and_then(|t| t.url).or_else(|| t.medium.and_then(|t| t.url)).or_else(|| t.default.and_then(|t| t.url)).or_else(|| t.standard.and_then(|t| t.url)).or_else(|| t.maxres.and_then(|t| t.url)) {
            return url;
        }
    }
    String::new()
}

#[derive(Clone, Debug)]
pub struct Subscription {
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub thumbnail_url: String,
}

#[derive(Clone, Debug)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub published_at: String,
    pub thumbnail_url: String,
    pub channel_title: String,
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub video_count: u32,
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub author_name: String,
    pub author_thumbnail: String,
    pub text_display: String,
    pub published_at: String,
    pub like_count: u32,
}


pub struct YoutubeClient {
    hub: YouTube<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>,
}

impl YoutubeClient {
    /// Create a new YouTubeClient by authenticating with OAuth2.
    /// It uses the desktop installed application flow and prompts the user in the browser if a cached token isn't found.
    pub async fn new_oauth(
        client_id: &str,
        client_secret: &str,
        token_cache_path: &Path,
    ) -> Result<Self> {
        Self::new_oauth_with_scopes(
            client_id,
            client_secret,
            token_cache_path,
            &["https://www.googleapis.com/auth/youtube.readonly"],
        ).await
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct OpenBrowserFlowDelegate;

impl yup_oauth2::authenticator_delegate::InstalledFlowDelegate for OpenBrowserFlowDelegate {
    fn present_user_url(
        &self,
        url: &str,
        _need_code: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<String, String>> + Send>> {
        let url_str = url.to_string();
        Box::pin(async move {
            println!("Opening browser for OAuth authentication: {}", url_str);
            if let Err(e) = open::that(&url_str) {
                eprintln!("Failed to open browser automatically: {}", e);
            }
            Ok(String::new())
        })
    }
}

impl YoutubeClient {
    pub async fn new_oauth_with_scopes(
        client_id: &str,
        client_secret: &str,
        token_cache_path: &Path,
        _scopes: &[&str],
    ) -> Result<Self> {
        let secret = ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
            ..Default::default()
        };

        let auth = InstalledFlowAuthenticator::builder(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect,
        )
        .flow_delegate(Box::new(OpenBrowserFlowDelegate))
        .persist_tokens_to_disk(token_cache_path)
        .build()
        .await?;

        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_only()
            .enable_http2()
            .build();

        let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(connector);

        let hub = YouTube::new(client, auth);

        Ok(Self { hub })
    }

    /// List the authenticated user's subscriptions.
    pub async fn list_subscriptions_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Subscription>, Option<String>)> {
        let limit = max_results.min(50);
        let (_response, list) = retry_api_call(|| async {
            let mut req = self.hub.subscriptions()
                .list(&vec!["snippet".to_string()])
                .mine(true)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        }).await?;
        let next_page_token = list.next_page_token;
        let mut subscriptions = Vec::new();
        if let Some(items) = list.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let id = item.id.unwrap_or_default();
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let channel_id = snippet.resource_id
                        .and_then(|r| r.channel_id)
                        .unwrap_or_default();
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    subscriptions.push(Subscription {
                        id,
                        title,
                        description,
                        channel_id,
                        thumbnail_url,
                    });
                }
            }
        }
        Ok((subscriptions, next_page_token))
    }

    /// List the authenticated user's subscriptions.
    pub async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        let (subs, _) = self.list_subscriptions_page(max_results, None).await?;
        Ok(subs)
    }

    /// List uploaded videos from a channel (usually a channel you're subscribed to).
    pub async fn list_videos_page(
        &self,
        channel_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Video>, Option<String>)> {
        // Step 1: Get the channel details to retrieve the uploads playlist ID
        let (_resp, channel_res) = retry_api_call(|| async {
            self.hub.channels()
                .list(&vec!["contentDetails".to_string()])
                .add_id(channel_id)
                .doit()
                .await
        }).await?;

        let items = channel_res.items.ok_or_else(|| YoutubeError::Other("Channel not found".to_string()))?;
        if items.is_empty() {
            return Err(YoutubeError::Other("Channel has no content details".to_string()));
        }
        let uploads_playlist_id = items[0]
            .content_details
            .as_ref()
            .and_then(|cd| cd.related_playlists.as_ref())
            .and_then(|rp| rp.uploads.as_ref())
            .ok_or_else(|| YoutubeError::Other("No uploads playlist found for this channel".to_string()))?;

        // Step 2: Fetch playlist items (videos) from the uploads playlist
        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self.hub.playlist_items()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .playlist_id(uploads_playlist_id)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        }).await?;

        let next_page_token = playlist_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = playlist_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item.content_details
                        .and_then(|cd| cd.video_id)
                        .unwrap_or_else(|| {
                            snippet.resource_id
                                .and_then(|r| r.video_id)
                                .unwrap_or_default()
                        });
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let published_at = snippet.published_at.unwrap_or_default();
                    let channel_title = snippet.channel_title.clone().unwrap_or_default();
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
                        channel_title,
                    });
                }
            }
        }
        Ok((videos, next_page_token))
    }

    /// List uploaded videos from a channel (usually a channel you're subscribed to).
    pub async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        let (vids, _) = self.list_videos_page(channel_id, max_results, None).await?;
        Ok(vids)
    }

    /// Search for videos using a query string.
    pub async fn search_videos_page(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Video>, Option<String>)> {
        let limit = max_results.min(50);
        let (_resp, search_res) = retry_api_call(|| async {
            let mut req = self.hub.search()
                .list(&vec!["snippet".to_string()])
                .q(query)
                .add_type("video")
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        }).await?;
        let next_page_token = search_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = search_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item.id
                        .and_then(|id| id.video_id)
                        .unwrap_or_default();
                    
                    if video_id.is_empty() {
                        continue;
                    }

                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let published_at = snippet.published_at.unwrap_or_default();
                    let channel_title = snippet.channel_title.clone().unwrap_or_default();
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
                        channel_title,
                    });
                }
            }
        }
        Ok((videos, next_page_token))
    }

    /// Search for videos using a query string.
    pub async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        let (vids, _) = self.search_videos_page(query, max_results, None).await?;
        Ok(vids)
    }

    /// List the authenticated user's playlists.
    pub async fn list_playlists_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Playlist>, Option<String>)> {
        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self.hub.playlists()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .mine(true)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        }).await?;
        let next_page_token = playlist_res.next_page_token;
        let mut playlists = Vec::new();
        if let Some(items) = playlist_res.items {
            for item in items {
                let id = item.id.unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                if let Some(snippet) = item.snippet {
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);
                    let video_count = item.content_details
                        .and_then(|cd| cd.item_count)
                        .unwrap_or(0);

                    playlists.push(Playlist {
                        id,
                        title,
                        description,
                        thumbnail_url,
                        video_count,
                    });
                }
            }
        }
        Ok((playlists, next_page_token))
    }

    /// List the authenticated user's playlists.
    pub async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        let (pls, _) = self.list_playlists_page(max_results, None).await?;
        Ok(pls)
    }

    /// List the videos inside a specific playlist.
    pub async fn list_playlist_videos_page(
        &self,
        playlist_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Video>, Option<String>)> {
        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self.hub.playlist_items()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .playlist_id(playlist_id)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        }).await?;

        let next_page_token = playlist_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = playlist_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item.content_details
                        .and_then(|cd| cd.video_id)
                        .unwrap_or_else(|| {
                            snippet.resource_id
                                .and_then(|r| r.video_id)
                                .unwrap_or_default()
                        });
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let published_at = snippet.published_at.unwrap_or_default();
                    let channel_title = snippet.video_owner_channel_title.clone()
                        .or_else(|| snippet.channel_title.clone())
                        .unwrap_or_default();
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
                        channel_title,
                    });
                }
            }
        }
        Ok((videos, next_page_token))
    }

    /// List the videos inside a specific playlist.
    pub async fn list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> Result<Vec<Video>> {
        let (vids, _) = self.list_playlist_videos_page(playlist_id, max_results, None).await?;
        Ok(vids)
    }

    /// Subscribe to a channel.
    pub async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        use google_youtube3::api::{Subscription as YtSub, SubscriptionSnippet, ResourceId};
        let snippet = SubscriptionSnippet {
            resource_id: Some(ResourceId {
                kind: Some("youtube#channel".to_string()),
                channel_id: Some(channel_id.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let sub = YtSub {
            snippet: Some(snippet),
            ..Default::default()
        };
        retry_api_call(|| async {
            self.hub.subscriptions().insert(sub.clone()).doit().await
        }).await?;
        Ok(())
    }

    /// Unsubscribe from a channel using its subscription ID.
    pub async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.subscriptions().delete(subscription_id).doit().await
        }).await?;
        Ok(())
    }

    /// Create a new playlist.
    pub async fn create_playlist(&self, title: &str, description: Option<&str>) -> Result<Playlist> {
        use google_youtube3::api::{Playlist as YtPlaylist, PlaylistSnippet};
        let snippet = PlaylistSnippet {
            title: Some(title.to_string()),
            description: description.map(|d| d.to_string()),
            ..Default::default()
        };
        let pl = YtPlaylist {
            snippet: Some(snippet),
            ..Default::default()
        };
        let (_resp, playlist_res) = retry_api_call(|| async {
            self.hub.playlists().insert(pl.clone()).doit().await
        }).await?;
        let id = playlist_res.id.unwrap_or_default();
        let title = playlist_res.snippet.as_ref().and_then(|s| s.title.clone()).unwrap_or_default();
        let description = playlist_res.snippet.as_ref().and_then(|s| s.description.clone()).unwrap_or_default();
        let thumbnail_url = playlist_res.snippet.as_ref().and_then(|s| s.thumbnails.clone()).and_then(|t| t.default).and_then(|t| t.url).unwrap_or_default();
        let video_count = playlist_res.content_details.and_then(|cd| cd.item_count).unwrap_or(0);
        
        Ok(Playlist {
            id,
            title,
            description,
            thumbnail_url,
            video_count,
        })
    }

    /// Add a video to a playlist.
    pub async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()> {
        use google_youtube3::api::{PlaylistItem, PlaylistItemSnippet, ResourceId};
        let snippet = PlaylistItemSnippet {
            playlist_id: Some(playlist_id.to_string()),
            resource_id: Some(ResourceId {
                kind: Some("youtube#video".to_string()),
                video_id: Some(video_id.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let item = PlaylistItem {
            snippet: Some(snippet),
            ..Default::default()
        };
        retry_api_call(|| async {
            self.hub.playlist_items().insert(item.clone()).doit().await
        }).await?;
        Ok(())
    }

    /// Remove a video from a playlist using its playlist item ID.
    pub async fn remove_from_playlist(&self, playlist_item_id: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.playlist_items().delete(playlist_item_id).doit().await
        }).await?;
        Ok(())
    }

    /// Rate a video ("like", "dislike", or "none").
    pub async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.videos().rate(video_id, rating).doit().await
        }).await?;
        Ok(())
    }

    /// Fetch top comment threads for a video.
    pub async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        let (_resp, comment_res) = retry_api_call(|| async {
            self.hub.comment_threads()
                .list(&vec!["snippet".to_string()])
                .video_id(video_id)
                .max_results(20)
                .doit()
                .await
        }).await?;

        let mut comments = Vec::new();
        if let Some(items) = comment_res.items {
            for item in items {
                if let Some(snippet) = item.snippet.and_then(|s| s.top_level_comment).and_then(|c| c.snippet) {
                    let author_name = snippet.author_display_name.unwrap_or_default();
                    let author_thumbnail = snippet.author_profile_image_url.unwrap_or_default();
                    let text_display = snippet.text_display.unwrap_or_default();
                    let published_at = snippet.published_at.unwrap_or_default();
                    let like_count = snippet.like_count.unwrap_or(0);

                    comments.push(Comment {
                        author_name,
                        author_thumbnail,
                        text_display,
                        published_at: published_at.to_string(),
                        like_count,
                    });
                }
            }
        }
        Ok(comments)
    }

    /// Download a YouTube video by ID to the target path.
    #[cfg(feature = "download")]
    pub async fn download_video<F>(&self, video_id: &str, output_path: &Path, on_progress: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        download_video_direct(video_id, output_path, on_progress).await
    }
}

/// Download a YouTube video directly by ID using yt-dlp.
#[cfg(feature = "download")]
pub async fn download_video_direct<F>(video_id: &str, output_path: &Path, on_progress: F) -> Result<()>
where
    F: Fn(&str) + Send + Sync + 'static,
{
    use tokio::io::AsyncReadExt;

    let url = format!("https://www.youtube.com/watch?v={}", video_id);
    let is_mp3 = output_path
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("mp3"));

    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.arg("--newline")
       .arg("--no-keep-video");
    if is_mp3 {
        cmd.arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("-o")
            .arg(output_path)
            .arg(&url);
    } else {
        cmd.arg("-f")
            .arg("bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]")
            .arg("-o")
            .arg(output_path)
            .arg(&url);
    }

    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(YoutubeError::YtDlpMissing(e.to_string()));
        }
        Err(e) => return Err(YoutubeError::Io(e)),
    };

    let mut stdout = child.stdout.take().ok_or_else(|| YoutubeError::Download("Failed to capture stdout".to_string()))?;
    let mut stderr = child.stderr.take().ok_or_else(|| YoutubeError::Download("Failed to capture stderr".to_string()))?;
    
    let stderr_handle = tokio::spawn(async move {
        let mut err_buf = Vec::new();
        let mut temp_err = [0u8; 1024];
        while let Ok(n) = stderr.read(&mut temp_err).await {
            if n == 0 {
                break;
            }
            err_buf.extend_from_slice(&temp_err[..n]);
        }
        String::from_utf8_lossy(&err_buf).into_owned()
    });
    
    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 1024];

    on_progress("Starting download...");

    loop {
        let n = stdout.read(&mut temp_buf).await?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp_buf[..n]);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n' || b == b'\r') {
            let line_bytes = buffer.drain(..pos + 1).collect::<Vec<u8>>();
            if line_bytes.is_empty() {
                continue;
            }
            let content_len = line_bytes.len() - 1;
            if let Ok(line_str) = std::str::from_utf8(&line_bytes[..content_len]) {
                let line = line_str.trim();
                if !line.is_empty() {
                    if line.contains("[download]") {
                        if let Some(pct_idx) = line.find('%') {
                            if let Some(dl_idx) = line.find("[download]") {
                                let start = dl_idx + 10;
                                if start < pct_idx {
                                    let pct = line[start..pct_idx].trim();
                                    on_progress(&format!("Downloading: {}%", pct));
                                }
                            }
                        } else if line.contains("Destination:") {
                            on_progress("Starting download...");
                        }
                    } else if line.contains("[ExtractAudio]") || line.contains("[ffmpeg]") {
                        on_progress("Extracting audio...");
                    }
                }
            }
        }
    }

    let status = child.wait().await?;
    let stderr_output = stderr_handle.await.unwrap_or_default();
    if !status.success() {
        return Err(YoutubeError::Download(format!("yt-dlp download failed: {}", stderr_output.trim())));
    }
    Ok(())
}

impl YoutubeClient {

    /// Play the audio of the downloaded video file using Rodio.
    #[cfg(feature = "audio")]
    pub async fn play_audio_rodio(file_path: std::path::PathBuf) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            use std::fs::File;
            use std::io::BufReader;
            use rodio::{Decoder, DeviceSinkBuilder, Player};

            let handle = DeviceSinkBuilder::open_default_sink()
                .map_err(|e| YoutubeError::Media(format!("Failed to open default audio stream: {:?}", e)))?;
            let player = Player::connect_new(&handle.mixer());

            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let source = Decoder::new(reader)
                .map_err(|e| YoutubeError::Media(format!("Failed to decode audio: {}", e)))?;

            player.append(source);
            player.play();
            player.sleep_until_end();
            Ok(())
        })
        .await
        .map_err(|e| YoutubeError::Other(format!("Audio thread panicked: {}", e)))?
    }

    /// Play the video using the system's default media player.
    pub fn play_video_system(&self, file_path: &Path) -> Result<()> {
        open::that(file_path).map_err(|e| YoutubeError::Media(e.to_string()))?;
        Ok(())
    }

    /// Test the connection to YouTube by trying to list a single subscription.
    pub async fn test_connection(&self) -> Result<()> {
        let _ = self.list_subscriptions(1).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_oauth_invalid_credentials() {
        // If we provide a path to a non-existent or dummy token cache, and invalid credentials,
        // it should fail to authenticate.
        // We write a dummy token cache containing expired token structure to ensure it attempts
        // a refresh and fails immediately rather than starting an interactive browser flow.
        let temp_dir = std::env::temp_dir();
        let cache_path = temp_dir.join("dummy_youtube_token_cache.json");
        
        let dummy_cache_content = r#"[
            {
                "scopes": ["https://www.googleapis.com/auth/youtube.readonly"],
                "token": {
                    "access_token": "dummy_access_token",
                    "refresh_token": "dummy_refresh_token",
                    "token_type": "Bearer",
                    "expires_at": "2020-01-01T00:00:00Z"
                }
            }
        ]"#;
        
        std::fs::write(&cache_path, dummy_cache_content).unwrap();

        let client = YoutubeClient::new_oauth(
            "invalid_client_id",
            "invalid_client_secret",
            &cache_path,
        ).await;

        // Clean up the dummy cache file
        let _ = std::fs::remove_file(&cache_path);

        // It must return an error since the refresh token is invalid and client ID/secret are invalid.
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_real_connection_from_config() {
        use std::path::PathBuf;

        // Since the current directory of tests in cargo is the crate root (youtube-client-lib),
        // we check for `private_config.json` and `config.json` relative to it, in the parent directory (../).
        let config = load_config_from_dir(Path::new(".."));
        let token_cache_path = PathBuf::from("../tokencache.json");

        if config.client_id.is_none() || config.client_secret.is_none() {
            println!("Skipping real connection test because no config file or client credentials exist.");
            return;
        }

        let client_id = config.client_id.unwrap();
        let client_secret = config.client_secret.unwrap();

        // If the client ID / secret are still placeholders, skip the test
        if client_id == "ENTER_YOUR_CLIENT_ID_HERE" || client_id.is_empty() {
            println!("Skipping real connection test because config contains placeholder values.");
            return;
        }

        println!("Running real YouTube connection test...");
        let client = YoutubeClient::new_oauth(
            &client_id,
            &client_secret,
            &token_cache_path,
        ).await;

        match client {
            Ok(client) => {
                let conn_result = client.test_connection().await;
                assert!(conn_result.is_ok(), "Failed to test connection to YouTube: {:?}", conn_result.err());
                println!("Successfully verified connection to YouTube!");
            }
            Err(e) => {
                panic!("Failed to initialize YoutubeClient with credentials from config.json: {:?}", e);
            }
        }
    }

    #[test]
    fn test_check_token_cache_scopes() {
        let temp_dir = std::env::temp_dir();
        let cache_path = temp_dir.join("test_check_token_cache_scopes.json");
        
        let dummy_cache_content = r#"[
            {
                "scopes": ["https://www.googleapis.com/auth/youtube.readonly"],
                "token": {}
            }
        ]"#;
        std::fs::write(&cache_path, dummy_cache_content).unwrap();

        assert!(check_token_cache_scopes(&cache_path, &["https://www.googleapis.com/auth/youtube.readonly"]));
        assert!(!check_token_cache_scopes(&cache_path, &["https://www.googleapis.com/auth/youtube"]));

        let _ = std::fs::remove_file(&cache_path);
    }

    #[test]
    fn test_load_config_from_file_or_default() {
        let temp_dir = std::env::temp_dir();
        let custom_config_path = temp_dir.join("test_custom_config.json");

        let custom_content = r#"{
            "client_id": "custom_id",
            "client_secret": "custom_secret"
        }"#;
        std::fs::write(&custom_config_path, custom_content).unwrap();

        // 1. Loads from custom config
        let cfg1 = load_config_from_file_or_default(&custom_config_path);
        assert_eq!(cfg1.client_id, Some("custom_id".to_string()));
        assert_eq!(cfg1.client_secret, Some("custom_secret".to_string()));

        // 2. Falls back to default config if config.json is passed
        let cfg2 = load_config_from_file_or_default(Path::new("config.json"));
        // This should run without error, returning default / loaded values
        let _ = cfg2.is_valid();

        let _ = std::fs::remove_file(&custom_config_path);
    }

    #[tokio::test]
    async fn test_retry_api_call_non_retryable() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));

        let res: std::result::Result<(), YoutubeError> = retry_api_call(|| {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err(google_youtube3::Error::MissingAPIKey)
            }
        }).await;

        assert!(res.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}

