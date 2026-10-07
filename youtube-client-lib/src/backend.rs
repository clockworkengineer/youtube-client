//! # Pluggable Backend Provider for YouTube Services
//!
//! Provides [`YoutubeBackend`], an interchangeable service adapter that can wrap either
//! a live Google API client ([`YoutubeClient`]) or an in-memory mock ([`MockYoutubeClient`]).
//!
//! Implements [`VideoService`], [`SubscriptionService`], [`PlaylistService`], [`CommentService`],
//! and [`YoutubeApiService`](crate::traits::YoutubeApiService).

use std::sync::Arc;

use crate::client::YoutubeClient;
use crate::download::DownloadOptions;
use crate::error::Result;
use crate::models::{ChannelDetails, Comment, Page, Playlist, Subscription, Video, VideoDetails};
use crate::testing::MockYoutubeClient;
use crate::traits::{
    CommentService, MediaDownloader, PlaylistService, SubscriptionService, VideoService,
};

/// An interchangeable backend provider that implements all YouTube domain service traits.
#[derive(Clone)]
pub enum YoutubeBackend {
    /// Live Google YouTube Data API v3 client.
    Live(Arc<YoutubeClient>),
    /// In-memory mock client for unit testing and offline execution.
    Mock(MockYoutubeClient),
}

impl YoutubeBackend {
    /// Create a backend wrapping a live client.
    pub fn live(client: YoutubeClient) -> Self {
        Self::Live(Arc::new(client))
    }

    /// Create a backend wrapping an Arc-shared live client.
    pub fn live_arc(client: Arc<YoutubeClient>) -> Self {
        Self::Live(client)
    }

    /// Create a backend wrapping an in-memory mock.
    pub fn mock(mock: MockYoutubeClient) -> Self {
        Self::Mock(mock)
    }

    /// Returns `true` if this backend is in-memory mock.
    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }

    /// List a page of subscriptions.
    pub async fn list_subscriptions_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Subscription>> {
        match self {
            Self::Live(c) => c.list_subscriptions_page(max_results, page_token).await,
            Self::Mock(m) => {
                let all = m.subscriptions.lock().unwrap();
                let offset: usize = page_token
                    .and_then(|t| t.strip_prefix("offset_"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let end = (offset + max_results as usize).min(all.len());
                let items = if offset < all.len() {
                    all[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next_token = if end < all.len() {
                    Some(format!("offset_{end}"))
                } else {
                    None
                };
                Ok(Page::new(items, next_token))
            }
        }
    }

    /// List subscriptions up to `max_results`.
    pub async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        SubscriptionService::list_subscriptions(self, max_results).await
    }

    /// Subscribe to a channel.
    pub async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        SubscriptionService::subscribe_to_channel(self, channel_id).await
    }

    /// Unsubscribe from a channel.
    pub async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        SubscriptionService::unsubscribe_from_channel(self, subscription_id).await
    }

    /// List a page of channel uploads.
    pub async fn list_videos_page(
        &self,
        channel_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        match self {
            Self::Live(c) => {
                c.list_videos_page(channel_id, max_results, page_token)
                    .await
            }
            Self::Mock(m) => {
                let vids = m.videos_by_channel.lock().unwrap();
                let all = vids.get(channel_id).cloned().unwrap_or_default();
                let offset: usize = page_token
                    .and_then(|t| t.strip_prefix("offset_"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let end = (offset + max_results as usize).min(all.len());
                let items = if offset < all.len() {
                    all[offset..end].to_vec()
                } else {
                    Vec::new()
                };
                let next_token = if end < all.len() {
                    Some(format!("offset_{end}"))
                } else {
                    None
                };
                Ok(Page::new(items, next_token))
            }
        }
    }

    /// List uploaded videos from a channel up to `max_results`.
    pub async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        VideoService::list_videos(self, channel_id, max_results).await
    }

    /// Search for videos with pagination.
    pub async fn search_videos_page(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        match self {
            Self::Live(c) => c.search_videos_page(query, max_results, page_token).await,
            Self::Mock(m) => {
                let items = m.search_videos(query, max_results).await?;
                Ok(Page::new(items, None))
            }
        }
    }

    /// Search videos matching query up to `max_results`.
    pub async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        VideoService::search_videos(self, query, max_results).await
    }

    /// Rate a video.
    pub async fn rate_video(&self, video_id: &str, rating: impl AsRef<str>) -> Result<()> {
        VideoService::rate_video(self, video_id, rating.as_ref()).await
    }

    /// Fetch detailed metadata for a video.
    pub async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails> {
        VideoService::fetch_video_details(self, video_id).await
    }

    /// List a page of user playlists.
    pub async fn list_playlists_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Playlist>> {
        match self {
            Self::Live(c) => c.list_playlists_page(max_results, page_token).await,
            Self::Mock(m) => {
                let items = m.list_playlists(max_results).await?;
                Ok(Page::new(items, None))
            }
        }
    }

    /// List playlists up to `max_results`.
    pub async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        PlaylistService::list_playlists(self, max_results).await
    }

    /// List a page of playlist videos.
    pub async fn list_playlist_videos_page(
        &self,
        playlist_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        match self {
            Self::Live(c) => {
                c.list_playlist_videos_page(playlist_id, max_results, page_token)
                    .await
            }
            Self::Mock(m) => {
                let items = m.list_playlist_videos(playlist_id, max_results).await?;
                Ok(Page::new(items, None))
            }
        }
    }

    /// List videos within a playlist up to `max_results`.
    pub async fn list_playlist_videos(
        &self,
        playlist_id: &str,
        max_results: u32,
    ) -> Result<Vec<Video>> {
        PlaylistService::list_playlist_videos(self, playlist_id, max_results).await
    }

    /// Add video to a playlist.
    pub async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()> {
        PlaylistService::add_to_playlist(self, playlist_id, video_id).await
    }

    /// Create a new playlist.
    pub async fn create_playlist(
        &self,
        title: &str,
        description: Option<&str>,
    ) -> Result<Playlist> {
        PlaylistService::create_playlist(self, title, description).await
    }

    /// Delete a playlist by ID.
    pub async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        PlaylistService::delete_playlist(self, playlist_id).await
    }

    /// Fetch comments for a video.
    pub async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        CommentService::fetch_comments(self, video_id).await
    }

    /// Post a comment to a video.
    pub async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment> {
        CommentService::post_comment(self, video_id, text).await
    }

    /// Fetch channel profile and subscriber statistics.
    pub async fn get_channel_details(&self, channel_id: &str) -> Result<ChannelDetails> {
        match self {
            Self::Live(c) => c.get_channel_details(channel_id).await,
            Self::Mock(m) => m.get_channel_details(channel_id).await,
        }
    }

    /// Fetch comprehensive video details for multiple videos in batches of up to 50.
    pub async fn get_videos_batch(&self, video_ids: &[&str]) -> Result<Vec<VideoDetails>> {
        VideoService::get_videos_batch(self, video_ids).await
    }

    /// Download media with options and progress callback.
    pub async fn download_video_with_options<F>(
        &self,
        video_id: &str,
        output_path: &std::path::Path,
        options: &DownloadOptions,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(crate::download::DownloadProgress) + Send + Sync + 'static,
    {
        match self {
            Self::Live(c) => {
                c.download_video_with_options(video_id, output_path, options, progress_callback)
                    .await
            }
            Self::Mock(m) => {
                progress_callback(crate::download::DownloadProgress::finished("Mock downloading: 100%"));
                let mut dls = m.downloaded_videos.lock().unwrap();
                dls.push(video_id.to_string());
                Ok(())
            }
        }
    }

    /// Stream subscriptions across pages as an asynchronous pinned [`BoxStream`](crate::client::BoxStream).
    pub fn stream_subscriptions(
        &self,
        page_size: u32,
    ) -> crate::client::BoxStream<'_, Result<Subscription>> {
        Box::pin(async_stream::try_stream! {
            let mut page_token = None;
            loop {
                let page = self.list_subscriptions_page(page_size, page_token.as_deref()).await?;
                for item in page.items {
                    yield item;
                }
                match page.next_page_token {
                    Some(token) if !token.is_empty() => page_token = Some(token),
                    _ => break,
                }
            }
        })
    }

    /// Stream uploaded videos from a channel as an asynchronous pinned [`BoxStream`](crate::client::BoxStream).
    pub fn stream_videos<'a>(
        &'a self,
        channel_id: &'a str,
        page_size: u32,
    ) -> crate::client::BoxStream<'a, Result<Video>> {
        Box::pin(async_stream::try_stream! {
            let mut page_token = None;
            loop {
                let page = self.list_videos_page(channel_id, page_size, page_token.as_deref()).await?;
                for item in page.items {
                    yield item;
                }
                match page.next_page_token {
                    Some(token) if !token.is_empty() => page_token = Some(token),
                    _ => break,
                }
            }
        })
    }

    /// Stream search results as an asynchronous pinned [`BoxStream`](crate::client::BoxStream).
    pub fn stream_search<'a>(
        &'a self,
        query: &'a str,
        page_size: u32,
    ) -> crate::client::BoxStream<'a, Result<Video>> {
        Box::pin(async_stream::try_stream! {
            let mut page_token = None;
            loop {
                let page = self.search_videos_page(query, page_size, page_token.as_deref()).await?;
                for item in page.items {
                    yield item;
                }
                match page.next_page_token {
                    Some(token) if !token.is_empty() => page_token = Some(token),
                    _ => break,
                }
            }
        })
    }

    /// Stream playlists as an asynchronous pinned [`BoxStream`](crate::client::BoxStream).
    pub fn stream_playlists(
        &self,
        page_size: u32,
    ) -> crate::client::BoxStream<'_, Result<Playlist>> {
        Box::pin(async_stream::try_stream! {
            let mut page_token = None;
            loop {
                let page = self.list_playlists_page(page_size, page_token.as_deref()).await?;
                for item in page.items {
                    yield item;
                }
                match page.next_page_token {
                    Some(token) if !token.is_empty() => page_token = Some(token),
                    _ => break,
                }
            }
        })
    }

    /// Stream videos from a playlist as an asynchronous pinned [`BoxStream`](crate::client::BoxStream).
    pub fn stream_playlist_videos<'a>(
        &'a self,
        playlist_id: &'a str,
        page_size: u32,
    ) -> crate::client::BoxStream<'a, Result<Video>> {
        Box::pin(async_stream::try_stream! {
            let mut page_token = None;
            loop {
                let page = self.list_playlist_videos_page(playlist_id, page_size, page_token.as_deref()).await?;
                for item in page.items {
                    yield item;
                }
                match page.next_page_token {
                    Some(token) if !token.is_empty() => page_token = Some(token),
                    _ => break,
                }
            }
        })
    }
}

impl MediaDownloader for YoutubeBackend {
    async fn download_media(
        &self,
        video_id: &str,
        output_path: &std::path::Path,
        progress_cb: Box<dyn Fn(&str) + Send + Sync>,
    ) -> Result<()> {
        match self {
            Self::Live(c) => c.download_media(video_id, output_path, progress_cb).await,
            Self::Mock(m) => m.download_media(video_id, output_path, progress_cb).await,
        }
    }
}

impl VideoService for YoutubeBackend {
    async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        match self {
            Self::Live(c) => c.list_videos(channel_id, max_results).await,
            Self::Mock(m) => m.list_videos(channel_id, max_results).await,
        }
    }

    async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        match self {
            Self::Live(c) => c.search_videos(query, max_results).await,
            Self::Mock(m) => m.search_videos(query, max_results).await,
        }
    }

    async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()> {
        match self {
            Self::Live(c) => c.rate_video(video_id, rating).await,
            Self::Mock(m) => m.rate_video(video_id, rating).await,
        }
    }

    async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails> {
        match self {
            Self::Live(c) => c.fetch_video_details(video_id).await,
            Self::Mock(m) => m.fetch_video_details(video_id).await,
        }
    }

    async fn get_videos_batch(&self, video_ids: &[&str]) -> Result<Vec<VideoDetails>> {
        match self {
            Self::Live(c) => c.get_videos_batch(video_ids).await,
            Self::Mock(m) => m.get_videos_batch(video_ids).await,
        }
    }
}

impl SubscriptionService for YoutubeBackend {
    async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        match self {
            Self::Live(c) => c.list_subscriptions(max_results).await,
            Self::Mock(m) => m.list_subscriptions(max_results).await,
        }
    }

    async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        match self {
            Self::Live(c) => c.subscribe_to_channel(channel_id).await,
            Self::Mock(m) => m.subscribe_to_channel(channel_id).await,
        }
    }

    async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        match self {
            Self::Live(c) => c.unsubscribe_from_channel(subscription_id).await,
            Self::Mock(m) => m.unsubscribe_from_channel(subscription_id).await,
        }
    }
}

impl PlaylistService for YoutubeBackend {
    async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        match self {
            Self::Live(c) => c.list_playlists(max_results).await,
            Self::Mock(m) => m.list_playlists(max_results).await,
        }
    }

    async fn list_playlist_videos(
        &self,
        playlist_id: &str,
        max_results: u32,
    ) -> Result<Vec<Video>> {
        match self {
            Self::Live(c) => c.list_playlist_videos(playlist_id, max_results).await,
            Self::Mock(m) => m.list_playlist_videos(playlist_id, max_results).await,
        }
    }

    async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()> {
        match self {
            Self::Live(c) => c.add_to_playlist(playlist_id, video_id).await,
            Self::Mock(m) => m.add_to_playlist(playlist_id, video_id).await,
        }
    }

    async fn create_playlist(&self, title: &str, description: Option<&str>) -> Result<Playlist> {
        match self {
            Self::Live(c) => c.create_playlist(title, description).await,
            Self::Mock(m) => m.create_playlist(title, description).await,
        }
    }

    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        match self {
            Self::Live(c) => c.delete_playlist(playlist_id).await,
            Self::Mock(m) => m.delete_playlist(playlist_id).await,
        }
    }
}

impl CommentService for YoutubeBackend {
    async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        match self {
            Self::Live(c) => c.fetch_comments(video_id).await,
            Self::Mock(m) => m.fetch_comments(video_id).await,
        }
    }

    async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment> {
        match self {
            Self::Live(c) => c.post_comment(video_id, text).await,
            Self::Mock(m) => m.post_comment(video_id, text).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend_operations() {
        let mock = MockYoutubeClient::new();
        let backend = YoutubeBackend::mock(mock);
        assert!(backend.is_mock());

        // Test SubscriptionService through backend
        assert!(backend.subscribe_to_channel("UC_test").await.is_ok());
        let subs = backend.list_subscriptions(10).await.unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel_id, "UC_test");

        // Test PlaylistService through backend
        let pl = backend
            .create_playlist("Test Playlist", None)
            .await
            .unwrap();
        assert_eq!(pl.title, "Test Playlist");

        let pls = backend.list_playlists(10).await.unwrap();
        assert_eq!(pls.len(), 1);

        // Test get_videos_batch through backend
        let batch_ids = vec!["vid_a", "vid_b", "vid_c"];
        let batch_details = backend.get_videos_batch(&batch_ids).await.unwrap();
        assert_eq!(batch_details.len(), 3);
        assert_eq!(batch_details[0].id, "vid_a");
        assert_eq!(batch_details[1].id, "vid_b");
        assert_eq!(batch_details[2].id, "vid_c");
    }
}
