//! # Mock Implementations for YouTube Service Traits
//!
//! Provides in-memory, mock implementations of [`VideoService`], [`SubscriptionService`],
//! [`PlaylistService`], [`CommentService`], and [`MediaDownloader`]. Useful for testing UI views
//! and CLI commands without real YouTube credentials or network access.

use crate::error::Result;
use crate::models::{Comment, Playlist, Subscription, Video, VideoDetails};
use crate::traits::{
    CommentService, MediaDownloader, PlaylistService, SubscriptionService, VideoService,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// An in-memory mock client that implements all library service traits.
#[derive(Clone, Default)]
pub struct MockYoutubeClient {
    pub subscriptions: Arc<Mutex<Vec<Subscription>>>,
    pub videos_by_channel: Arc<Mutex<HashMap<String, Vec<Video>>>>,
    pub playlists: Arc<Mutex<Vec<Playlist>>>,
    pub playlist_items: Arc<Mutex<HashMap<String, Vec<Video>>>>,
    pub comments_by_video: Arc<Mutex<HashMap<String, Vec<Comment>>>>,
    pub downloaded_videos: Arc<Mutex<Vec<String>>>,
    pub video_ratings: Arc<Mutex<HashMap<String, String>>>,
}

impl MockYoutubeClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SubscriptionService for MockYoutubeClient {
    async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        let subs = self.subscriptions.lock().unwrap();
        Ok(subs.iter().take(max_results as usize).cloned().collect())
    }

    async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        let mut subs = self.subscriptions.lock().unwrap();
        subs.push(Subscription {
            id: format!("sub_{channel_id}"),
            title: format!("Channel {channel_id}"),
            description: "Mock channel".to_string(),
            channel_id: channel_id.to_string(),
            thumbnail_url: String::new(),
        });
        Ok(())
    }

    async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        let mut subs = self.subscriptions.lock().unwrap();
        subs.retain(|s| s.id != subscription_id);
        Ok(())
    }
}

impl VideoService for MockYoutubeClient {
    async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        let map = self.videos_by_channel.lock().unwrap();
        let list = map.get(channel_id).cloned().unwrap_or_default();
        Ok(list.into_iter().take(max_results as usize).collect())
    }

    async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        let map = self.videos_by_channel.lock().unwrap();
        let mut results = Vec::new();
        for vids in map.values() {
            for v in vids {
                if v.title.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(v.clone());
                }
            }
        }
        Ok(results.into_iter().take(max_results as usize).collect())
    }

    async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()> {
        let mut ratings = self.video_ratings.lock().unwrap();
        ratings.insert(video_id.to_string(), rating.to_string());
        Ok(())
    }

    async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails> {
        Ok(VideoDetails {
            id: video_id.to_string(),
            title: format!("Mock Video {video_id}"),
            description: "Mock video description".to_string(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            channel_id: "mock_channel".to_string(),
            channel_title: "Mock Channel".to_string(),
            thumbnail_url: String::new(),
            view_count: 12345,
            like_count: 678,
            comment_count: 90,
            duration_seconds: 300,
            duration_formatted: "5:00".to_string(),
            tags: vec!["mock".to_string(), "video".to_string()],
        })
    }
}

impl PlaylistService for MockYoutubeClient {
    async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        let pls = self.playlists.lock().unwrap();
        Ok(pls.iter().take(max_results as usize).cloned().collect())
    }

    async fn list_playlist_videos(
        &self,
        playlist_id: &str,
        max_results: u32,
    ) -> Result<Vec<Video>> {
        let map = self.playlist_items.lock().unwrap();
        let list = map.get(playlist_id).cloned().unwrap_or_default();
        Ok(list.into_iter().take(max_results as usize).collect())
    }

    async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()> {
        let mut map = self.playlist_items.lock().unwrap();
        let list = map.entry(playlist_id.to_string()).or_default();
        list.push(Video {
            id: video_id.to_string(),
            title: format!("Video {video_id}"),
            description: String::new(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            thumbnail_url: String::new(),
            channel_title: "Mock Channel".to_string(),
        });
        Ok(())
    }

    async fn create_playlist(&self, title: &str, description: Option<&str>) -> Result<Playlist> {
        let mut pls = self.playlists.lock().unwrap();
        let pl = Playlist {
            id: format!("pl_{}", pls.len() + 1),
            title: title.to_string(),
            description: description.unwrap_or_default().to_string(),
            thumbnail_url: String::new(),
            video_count: 0,
        };
        pls.push(pl.clone());
        Ok(pl)
    }

    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        let mut pls = self.playlists.lock().unwrap();
        pls.retain(|p| p.id != playlist_id);
        Ok(())
    }
}

impl CommentService for MockYoutubeClient {
    async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        let map = self.comments_by_video.lock().unwrap();
        Ok(map.get(video_id).cloned().unwrap_or_default())
    }

    async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment> {
        let mut map = self.comments_by_video.lock().unwrap();
        let list = map.entry(video_id.to_string()).or_default();
        let comment = Comment {
            author_name: "Mock User".to_string(),
            author_thumbnail: String::new(),
            text_display: text.to_string(),
            published_at: "Just now".to_string(),
            like_count: 0,
        };
        list.push(comment.clone());
        Ok(comment)
    }
}

impl MediaDownloader for MockYoutubeClient {
    async fn download_media(
        &self,
        video_id: &str,
        _output_path: &Path,
        progress_cb: Box<dyn Fn(&str) + Send + Sync>,
    ) -> Result<()> {
        progress_cb("Mock downloading: 100%");
        let mut dls = self.downloaded_videos.lock().unwrap();
        dls.push(video_id.to_string());
        Ok(())
    }
}
