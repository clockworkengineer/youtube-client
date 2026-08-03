use std::path::Path;
use google_youtube3::{YouTube, hyper_rustls, hyper_util};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};

#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub player_path: Option<String>,
    pub downloads_dir: Option<String>,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        let invalid_id = |id: &str| id.is_empty() || id == "ENTER_YOUR_CLIENT_ID_HERE";
        let invalid_secret = |sec: &str| sec.is_empty() || sec == "ENTER_YOUR_CLIENT_SECRET_HERE";
        
        self.client_id.as_deref().map_or(false, |id| !invalid_id(id))
            && self.client_secret.as_deref().map_or(false, |sec| !invalid_secret(sec))
    }
}

pub const YOUTUBE_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/youtube",
    "https://www.googleapis.com/auth/youtube.force-ssl",
    "https://www.googleapis.com/auth/youtube.readonly",
];

pub const GOOGLE_SETUP_INSTRUCTIONS: &str = "\
Please configure them in one of the following ways:\n\
1. Pass them as arguments: --client-id <ID> --client-secret <SECRET>\n\
2. Set the GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables\n\
3. Create a config file (config.json) with client credentials, e.g.:\n\
   {\n\
     \"client_id\": \"your_id_here\",\n\
     \"client_secret\": \"your_secret_here\"\n\
   }\n\n\
To get Google API Client credentials:\n\
1. Go to the Google Cloud Console: https://console.cloud.google.com/\n\
2. Create a project and search for the \"YouTube Data API v3\" and enable it.\n\
3. Navigate to \"APIs & Services\" > \"Credentials\".\n\
4. Click \"Create Credentials\" > \"OAuth client ID\". Choose \"Desktop app\".\n\
5. Retrieve your Client ID and Client Secret.";

pub fn load_config() -> Config {
    load_config_from_dir(Path::new("."))
}

pub fn load_config_from_dir(dir: &Path) -> Config {
    let private_config = dir.join("private_config.json");
    let fallback_config = dir.join("config.json");

    let config_path = if private_config.exists() {
        private_config
    } else {
        fallback_config
    };

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<Config>(&content) {
                return config;
            }
        }
    }
    Config::default()
}

pub fn check_token_cache_scopes(token_cache_path: &Path, required_scopes: &[&str]) -> bool {
    if let Ok(content) = std::fs::read_to_string(token_cache_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            let check_scopes = |scopes: &serde_json::Value| -> bool {
                if let Some(arr) = scopes.as_array() {
                    for scope in arr {
                        if let Some(scope_str) = scope.as_str() {
                            if required_scopes.iter().any(|&s| s == scope_str) {
                                return true;
                            }
                        }
                    }
                }
                false
            };

            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(scopes) = item.get("scopes") {
                        if check_scopes(scopes) {
                            return true;
                        }
                    }
                }
            } else if let Some(obj) = val.as_object() {
                for (_k, v) in obj {
                    if let Some(scopes) = v.get("scopes") {
                        if check_scopes(scopes) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}


fn extract_thumbnail_url(thumbnails: Option<google_youtube3::api::ThumbnailDetails>) -> String {
    thumbnails
        .and_then(|t| t.default)
        .and_then(|t| t.url)
        .unwrap_or_default()
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
    ) -> anyhow::Result<Self> {
        Self::new_oauth_with_scopes(
            client_id,
            client_secret,
            token_cache_path,
            &["https://www.googleapis.com/auth/youtube.readonly"],
        ).await
    }

    pub async fn new_oauth_with_scopes(
        client_id: &str,
        client_secret: &str,
        token_cache_path: &Path,
        scopes: &[&str],
    ) -> anyhow::Result<Self> {
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
        .persist_tokens_to_disk(token_cache_path)
        .build()
        .await?;

        // Warm up / ensure the token is retrieved/cached
        let _token = auth.token(scopes).await?;

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
    pub async fn list_subscriptions(&self, max_results: u32) -> anyhow::Result<Vec<Subscription>> {
        let limit = max_results.min(50);
        let req = self.hub.subscriptions()
            .list(&vec!["snippet".to_string()])
            .mine(true)
            .max_results(limit);

        let (_response, list) = req.doit().await?;
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
        Ok(subscriptions)
    }

    /// List uploaded videos from a channel (usually a channel you're subscribed to).
    pub async fn list_videos(&self, channel_id: &str, max_results: u32) -> anyhow::Result<Vec<Video>> {
        // Step 1: Get the channel details to retrieve the uploads playlist ID
        let (_resp, channel_res) = self.hub.channels()
            .list(&vec!["contentDetails".to_string()])
            .add_id(channel_id)
            .doit()
            .await?;

        let items = channel_res.items.ok_or_else(|| anyhow::anyhow!("Channel not found"))?;
        if items.is_empty() {
            return Err(anyhow::anyhow!("Channel has no content details"));
        }
        let uploads_playlist_id = items[0]
            .content_details
            .as_ref()
            .and_then(|cd| cd.related_playlists.as_ref())
            .and_then(|rp| rp.uploads.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No uploads playlist found for this channel"))?;

        // Step 2: Fetch playlist items (videos) from the uploads playlist
        let limit = max_results.min(50);
        let (_resp, playlist_res) = self.hub.playlist_items()
            .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
            .playlist_id(uploads_playlist_id)
            .max_results(limit)
            .doit()
            .await?;

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
        Ok(videos)
    }

    /// Search for videos using a query string.
    pub async fn search_videos(&self, query: &str, max_results: u32) -> anyhow::Result<Vec<Video>> {
        let limit = max_results.min(50);
        let req = self.hub.search()
            .list(&vec!["snippet".to_string()])
            .q(query)
            .add_type("video")
            .max_results(limit);

        let (_resp, search_res) = req.doit().await?;
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
        Ok(videos)
    }

    /// List the authenticated user's playlists.
    pub async fn list_playlists(&self, max_results: u32) -> anyhow::Result<Vec<Playlist>> {
        let limit = max_results.min(50);
        let req = self.hub.playlists()
            .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
            .mine(true)
            .max_results(limit);

        let (_resp, playlist_res) = req.doit().await?;
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
        Ok(playlists)
    }

    /// List the videos inside a specific playlist.
    pub async fn list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> anyhow::Result<Vec<Video>> {
        let limit = max_results.min(50);
        let (_resp, playlist_res) = self.hub.playlist_items()
            .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
            .playlist_id(playlist_id)
            .max_results(limit)
            .doit()
            .await?;

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
        Ok(videos)
    }

    /// Subscribe to a channel.
    pub async fn subscribe_to_channel(&self, channel_id: &str) -> anyhow::Result<()> {
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
        self.hub.subscriptions().insert(sub).doit().await?;
        Ok(())
    }

    /// Unsubscribe from a channel using its subscription ID.
    pub async fn unsubscribe_from_channel(&self, subscription_id: &str) -> anyhow::Result<()> {
        self.hub.subscriptions().delete(subscription_id).doit().await?;
        Ok(())
    }

    /// Create a new playlist.
    pub async fn create_playlist(&self, title: &str, description: Option<&str>) -> anyhow::Result<Playlist> {
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
        let (_resp, playlist_res) = self.hub.playlists().insert(pl).doit().await?;
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
    pub async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> anyhow::Result<()> {
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
        self.hub.playlist_items().insert(item).doit().await?;
        Ok(())
    }

    /// Remove a video from a playlist using its playlist item ID.
    pub async fn remove_from_playlist(&self, playlist_item_id: &str) -> anyhow::Result<()> {
        self.hub.playlist_items().delete(playlist_item_id).doit().await?;
        Ok(())
    }

    /// Rate a video ("like", "dislike", or "none").
    pub async fn rate_video(&self, video_id: &str, rating: &str) -> anyhow::Result<()> {
        self.hub.videos().rate(video_id, rating).doit().await?;
        Ok(())
    }

    /// Fetch top comment threads for a video.
    pub async fn fetch_comments(&self, video_id: &str) -> anyhow::Result<Vec<Comment>> {
        let (_resp, comment_res) = self.hub.comment_threads()
            .list(&vec!["snippet".to_string()])
            .video_id(video_id)
            .max_results(20)
            .doit()
            .await?;

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
    pub async fn download_video<F>(&self, video_id: &str, output_path: &Path, on_progress: F) -> anyhow::Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        download_video_direct(video_id, output_path, on_progress).await
    }
}

/// Download a YouTube video directly by ID using yt-dlp.
#[cfg(feature = "download")]
pub async fn download_video_direct<F>(video_id: &str, output_path: &Path, on_progress: F) -> anyhow::Result<()>
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

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;
    
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
        anyhow::bail!("yt-dlp download failed: {}", stderr_output.trim());
    }
    Ok(())
}

impl YoutubeClient {



    /// Play the audio of the downloaded video file using Rodio.
    #[cfg(feature = "audio")]
    pub fn play_audio_rodio(file_path: &Path) -> anyhow::Result<()> {
        use std::fs::File;
        use std::io::BufReader;
        use rodio::{Decoder, DeviceSinkBuilder, Player};

        let handle = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| anyhow::anyhow!("Failed to open default audio stream: {:?}", e))?;
        let player = Player::connect_new(&handle.mixer());

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)
            .map_err(|e| anyhow::anyhow!("Failed to decode audio: {}", e))?;

        player.append(source);
        player.sleep_until_end();
        Ok(())
    }

    /// Play the video using the system's default media player.
    pub fn play_video_system(&self, file_path: &Path) -> anyhow::Result<()> {
        open::that(file_path)?;
        Ok(())
    }

    /// Test the connection to YouTube by trying to list a single subscription.
    pub async fn test_connection(&self) -> anyhow::Result<()> {
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
}

