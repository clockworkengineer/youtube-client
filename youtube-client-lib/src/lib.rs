use std::path::Path;
use google_youtube3::{YouTube, hyper_rustls, hyper_util};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};

#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub player_path: Option<String>,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        let invalid_id = |id: &str| id.is_empty() || id == "ENTER_YOUR_CLIENT_ID_HERE";
        let invalid_secret = |sec: &str| sec.is_empty() || sec == "ENTER_YOUR_CLIENT_SECRET_HERE";
        
        self.client_id.as_deref().map_or(false, |id| !invalid_id(id))
            && self.client_secret.as_deref().map_or(false, |sec| !invalid_secret(sec))
    }
}

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
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub video_count: u32,
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
                    
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);


                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
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
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
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
                    let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

                    videos.push(Video {
                        id: video_id,
                        title,
                        description,
                        published_at: published_at.to_string(),
                        thumbnail_url,
                    });
                }
            }
        }
        Ok(videos)
    }



    /// Download a YouTube video by ID to the target path.
    #[cfg(feature = "download")]
    pub async fn download_video(&self, video_id: &str, output_path: &Path) -> anyhow::Result<()> {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);
        let video = rusty_ytdl::Video::new(url)?;
        video.download(output_path).await?;
        Ok(())
    }

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
}

