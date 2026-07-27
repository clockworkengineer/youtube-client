use std::path::Path;
use google_youtube3::{YouTube, hyper_rustls, hyper_util};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};

pub struct Subscription {
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
}

pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub published_at: String,
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
        let secret = ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
            ..Default::default()
        };

        // We use the youtube.readonly scope to read subscriptions and videos
        let scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

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
                    subscriptions.push(Subscription {
                        id,
                        title,
                        description,
                        channel_id,
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
                    
                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                    });
                }
            }
        }
        Ok(videos)
    }

    /// Download a YouTube video by ID to the target path.
    pub async fn download_video(&self, video_id: &str, output_path: &Path) -> anyhow::Result<()> {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        let video = rusty_ytdl::Video::new(url)?;
        video.download(output_path).await?;
        Ok(())
    }

    /// Play the audio of the downloaded video file using Rodio.
    pub fn play_audio_rodio(&self, file_path: &Path) -> anyhow::Result<()> {
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
}
