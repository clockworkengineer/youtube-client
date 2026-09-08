//! # YouTube Client Library (`youtube-client-lib`)
//!
//! A modular, asynchronous Rust library for interacting with the YouTube Data API v3,
//! managing subscriptions, browsing uploaded videos and playlists, streaming audio via Rodio,
//! and downloading media streams via yt-dlp.
//!
//! ## Architecture Overview
//!
//! The library is organized according to the 10 Attributes of a Well-Written Software Library:
//! - **[`client::YoutubeClient`]**: Core API client with fluent construction via [`builder::YoutubeClientBuilder`].
//! - **[`traits`]**: Segregated service traits (`VideoService`, `SubscriptionService`, `PlaylistService`, `CommentService`, `MediaDownloader`) for polymorphic dependency injection.
//! - **[`models`]**: Rich domain types including [`models::Video`], [`models::Playlist`], [`models::Subscription`], [`models::Comment`], typed [`models::Rating`], and generic [`models::Page`].
//! - **[`config`]**: Configuration loading with cross-platform fallback directories and secure credential storage.
//! - **[`utils`]**: Security-hardened filename sanitization, atomic JSON persistence, and cross-platform media player dispatching.
//! - **[`testing`]**: In-memory [`testing::MockYoutubeClient`] for testing client applications without real YouTube API credentials or network access.
//!
//! ## Quickstart Example
//!
//! ```no_run
//! use youtube_client_lib::{YoutubeClient, VideoService, Rating};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = YoutubeClient::builder()
//!         .with_credentials("MY_CLIENT_ID", "MY_CLIENT_SECRET")
//!         .with_token_cache("tokencache.json")
//!         .build()
//!         .await?;
//!
//!     let videos = client.list_videos("UC_x5XG1OV2P6uZZ5FSM9Ttw", 10).await?;
//!     for video in videos {
//!         println!("Title: {}", video.title);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod audio;
pub mod auth;
pub mod builder;
pub mod client;
pub mod config;
pub mod download;
pub mod error;
pub mod models;
pub mod retry;
pub mod testing;
pub mod traits;
pub mod utils;

pub use audio::*;
pub use auth::*;
pub use builder::YoutubeClientBuilder;
pub use client::*;
pub use config::*;
pub use download::*;
pub use error::*;
pub use models::*;
pub use traits::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_new_oauth_invalid_credentials() {
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
        )
        .await;

        let _ = std::fs::remove_file(&cache_path);
        assert!(client.is_err());
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

        assert!(check_token_cache_scopes(
            &cache_path,
            &["https://www.googleapis.com/auth/youtube.readonly"]
        ));
        assert!(!check_token_cache_scopes(
            &cache_path,
            &["https://www.googleapis.com/auth/youtube"]
        ));

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

        let cfg1 = load_config_from_file_or_default(&custom_config_path);
        assert_eq!(cfg1.client_id, Some("custom_id".to_string()));
        assert_eq!(cfg1.client_secret, Some("custom_secret".to_string()));

        let cfg2 = load_config_from_file_or_default(Path::new("config.json"));
        let _ = cfg2.is_valid();

        let _ = std::fs::remove_file(&custom_config_path);
    }

    #[tokio::test]
    async fn test_retry_api_call_non_retryable() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));

        let res: std::result::Result<(), YoutubeError> = retry::retry_api_call(|| {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err(google_youtube3::Error::MissingAPIKey)
            }
        })
        .await;

        assert!(res.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_rating_parsing() {
        use std::str::FromStr;
        assert_eq!(Rating::from_str("like").unwrap(), Rating::Like);
        assert_eq!(Rating::from_str("DISLIKE").unwrap(), Rating::Dislike);
        assert_eq!(Rating::from_str("none").unwrap(), Rating::None);
        assert!(Rating::from_str("invalid").is_err());
    }

    #[test]
    fn test_page_helpers() {
        let page = Page::new(vec![1, 2, 3], Some("next_token".to_string()));
        assert_eq!(page.len(), 3);
        assert!(!page.is_empty());
        assert!(page.has_more());

        let last_page = Page::new(vec![4], None);
        assert_eq!(last_page.len(), 1);
        assert!(!last_page.has_more());
    }
}
