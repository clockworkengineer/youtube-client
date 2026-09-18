//! # Core YouTube Client Implementation

use std::path::Path;
use google_youtube3::{hyper_rustls, hyper_util, YouTube};
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;
use yup_oauth2::{ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

use crate::auth::delegate::OpenBrowserFlowDelegate;
use crate::builder::YoutubeClientBuilder;
use crate::config::resolve_credentials;
use crate::error::{Result, YoutubeError};
use crate::models::{
    ChannelDetails, Comment, Page, Playlist, Rating, Subscription, Video, VideoDetails,
};
use crate::retry::retry_api_call;
use crate::traits::{
    CommentService, MediaDownloader, PlaylistService, SubscriptionService, VideoService,
};

/// Default scopes requested for full YouTube read/write client features.
pub const YOUTUBE_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/youtube",
    "https://www.googleapis.com/auth/youtube.force-ssl",
    "https://www.googleapis.com/auth/youtube.readonly",
];

fn extract_thumbnail_url(thumbnails: Option<google_youtube3::api::ThumbnailDetails>) -> String {
    if let Some(t) = thumbnails {
        if let Some(url) = t
            .high
            .and_then(|t| t.url)
            .or_else(|| t.medium.and_then(|t| t.url))
            .or_else(|| t.default.and_then(|t| t.url))
            .or_else(|| t.standard.and_then(|t| t.url))
            .or_else(|| t.maxres.and_then(|t| t.url))
        {
            return url;
        }
    }
    String::new()
}

/// The primary client for interacting with the YouTube API v3.
pub struct YoutubeClient {
    hub: YouTube<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>,
}

impl YoutubeClient {
    /// Create a fluent builder for configuring and instantiating a [`YoutubeClient`].
    pub fn builder() -> YoutubeClientBuilder {
        YoutubeClientBuilder::new()
    }

    /// Create a new `YoutubeClient` with read-only OAuth scopes.
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
        )
        .await
    }

    /// Create a new `YoutubeClient` with custom OAuth scopes.
    pub async fn new_oauth_with_scopes(
        client_id: &str,
        client_secret: &str,
        token_cache_path: &Path,
        scopes: &[&str],
    ) -> Result<Self> {
        Self::construct_with_params(
            client_id,
            client_secret,
            token_cache_path,
            scopes,
            InstalledFlowReturnMethod::HTTPRedirect,
            Box::new(OpenBrowserFlowDelegate),
        )
        .await
    }

    pub(crate) async fn construct_with_params(
        client_id: &str,
        client_secret: &str,
        token_cache_path: &Path,
        scopes: &[&str],
        return_method: InstalledFlowReturnMethod,
        delegate: Box<dyn InstalledFlowDelegate>,
    ) -> Result<Self> {
        let secret = ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
            ..Default::default()
        };

        let auth = InstalledFlowAuthenticator::builder(secret, return_method)
            .flow_delegate(delegate)
            .persist_tokens_to_disk(token_cache_path)
            .build()
            .await?;

        if !scopes.is_empty() {
            auth.token(scopes).await?;
        }

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

    /// List a page of the authenticated user's subscriptions.
    pub async fn list_subscriptions_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Subscription>> {
        let limit = max_results.min(50);
        let (_response, list) = retry_api_call(|| async {
            let mut req = self
                .hub
                .subscriptions()
                .list(&vec!["snippet".to_string()])
                .mine(true)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        })
        .await?;

        let next_page_token = list.next_page_token;
        let mut subscriptions = Vec::new();
        if let Some(items) = list.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let id = item.id.unwrap_or_default();
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let channel_id = snippet
                        .resource_id
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
        Ok(Page::new(subscriptions, next_page_token))
    }

    /// List the authenticated user's subscriptions up to `max_results`.
    pub async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        let page = self.list_subscriptions_page(max_results, None).await?;
        Ok(page.items)
    }

    /// List uploaded videos from a channel with pagination.
    pub async fn list_videos_page(
        &self,
        channel_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        let (_resp, channel_res) = retry_api_call(|| async {
            self.hub
                .channels()
                .list(&vec!["contentDetails".to_string()])
                .add_id(channel_id)
                .doit()
                .await
        })
        .await?;

        let items = channel_res
            .items
            .ok_or_else(|| YoutubeError::Other("Channel not found".to_string()))?;
        if items.is_empty() {
            return Err(YoutubeError::Other("Channel has no content details".to_string()));
        }
        let uploads_playlist_id = items[0]
            .content_details
            .as_ref()
            .and_then(|cd| cd.related_playlists.as_ref())
            .and_then(|rp| rp.uploads.as_ref())
            .ok_or_else(|| YoutubeError::Other("No uploads playlist found for this channel".to_string()))?;

        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self
                .hub
                .playlist_items()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .playlist_id(uploads_playlist_id)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        })
        .await?;

        let next_page_token = playlist_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = playlist_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item
                        .content_details
                        .and_then(|cd| cd.video_id)
                        .unwrap_or_else(|| {
                            snippet
                                .resource_id
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
        Ok(Page::new(videos, next_page_token))
    }

    /// List uploaded videos from a channel.
    pub async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        let page = self.list_videos_page(channel_id, max_results, None).await?;
        Ok(page.items)
    }

    /// Search for videos using a query string with pagination.
    pub async fn search_videos_page(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        let limit = max_results.min(50);
        let (_resp, search_res) = retry_api_call(|| async {
            let mut req = self
                .hub
                .search()
                .list(&vec!["snippet".to_string()])
                .q(query)
                .add_type("video")
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        })
        .await?;

        let next_page_token = search_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = search_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item.id.and_then(|id| id.video_id).unwrap_or_default();
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
        Ok(Page::new(videos, next_page_token))
    }

    /// Search for videos using a query string.
    pub async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        let page = self.search_videos_page(query, max_results, None).await?;
        Ok(page.items)
    }

    /// Fetch comprehensive video details including engagement metrics, duration, and tags.
    pub async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails> {
        let (_resp, video_res) = retry_api_call(|| async {
            self.hub
                .videos()
                .list(&vec![
                    "snippet".to_string(),
                    "statistics".to_string(),
                    "contentDetails".to_string(),
                ])
                .add_id(video_id)
                .doit()
                .await
        })
        .await?;

        let item = video_res
            .items
            .and_then(|items| items.into_iter().next())
            .ok_or_else(|| YoutubeError::Other(format!("Video '{}' not found", video_id)))?;

        let snippet = item.snippet.unwrap_or_default();
        let stats = item.statistics.unwrap_or_default();
        let content_details = item.content_details.unwrap_or_default();

        let title = snippet.title.unwrap_or_default();
        let description = snippet.description.unwrap_or_default();
        let published_at = snippet.published_at.map(|dt| dt.to_string()).unwrap_or_default();
        let channel_id = snippet.channel_id.unwrap_or_default();
        let channel_title = snippet.channel_title.unwrap_or_default();
        let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);
        let tags = snippet.tags.unwrap_or_default();

        let view_count = stats.view_count.unwrap_or(0);
        let like_count = stats.like_count.unwrap_or(0);
        let comment_count = stats.comment_count.unwrap_or(0);

        let duration_iso = content_details.duration.unwrap_or_default();
        let duration_seconds = VideoDetails::parse_iso8601_duration(&duration_iso);
        let duration_formatted = VideoDetails::format_duration(duration_seconds);

        Ok(VideoDetails {
            id: video_id.to_string(),
            title,
            description,
            published_at,
            channel_id,
            channel_title,
            thumbnail_url,
            view_count,
            like_count,
            comment_count,
            duration_seconds,
            duration_formatted,
            tags,
        })
    }

    /// Fetch channel statistics and profile metadata.
    pub async fn get_channel_details(&self, channel_id: &str) -> Result<ChannelDetails> {
        let (_resp, channel_res) = retry_api_call(|| async {
            self.hub
                .channels()
                .list(&vec![
                    "snippet".to_string(),
                    "statistics".to_string(),
                ])
                .add_id(channel_id)
                .doit()
                .await
        })
        .await?;

        let item = channel_res
            .items
            .and_then(|items| items.into_iter().next())
            .ok_or_else(|| YoutubeError::Other(format!("Channel '{}' not found", channel_id)))?;

        let snippet = item.snippet.unwrap_or_default();
        let stats = item.statistics.unwrap_or_default();

        let title = snippet.title.unwrap_or_default();
        let description = snippet.description.unwrap_or_default();
        let custom_url = snippet.custom_url;
        let thumbnail_url = extract_thumbnail_url(snippet.thumbnails);

        let subscriber_count = stats.subscriber_count.unwrap_or(0);
        let video_count = stats.video_count.unwrap_or(0);
        let view_count = stats.view_count.unwrap_or(0);

        Ok(ChannelDetails {
            id: channel_id.to_string(),
            title,
            description,
            custom_url,
            thumbnail_url,
            subscriber_count,
            video_count,
            view_count,
        })
    }

    /// List the authenticated user's playlists with pagination.
    pub async fn list_playlists_page(
        &self,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Playlist>> {
        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self
                .hub
                .playlists()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .mine(true)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        })
        .await?;

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
                    let video_count = item
                        .content_details
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
        Ok(Page::new(playlists, next_page_token))
    }

    /// List the authenticated user's playlists.
    pub async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        let page = self.list_playlists_page(max_results, None).await?;
        Ok(page.items)
    }

    /// List the videos inside a specific playlist with pagination.
    pub async fn list_playlist_videos_page(
        &self,
        playlist_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Video>> {
        let limit = max_results.min(50);
        let (_resp, playlist_res) = retry_api_call(|| async {
            let mut req = self
                .hub
                .playlist_items()
                .list(&vec!["snippet".to_string(), "contentDetails".to_string()])
                .playlist_id(playlist_id)
                .max_results(limit);
            if let Some(token) = page_token {
                req = req.page_token(token);
            }
            req.doit().await
        })
        .await?;

        let next_page_token = playlist_res.next_page_token;
        let mut videos = Vec::new();
        if let Some(items) = playlist_res.items {
            for item in items {
                if let Some(snippet) = item.snippet {
                    let video_id = item
                        .content_details
                        .and_then(|cd| cd.video_id)
                        .unwrap_or_else(|| {
                            snippet
                                .resource_id
                                .and_then(|r| r.video_id)
                                .unwrap_or_default()
                        });
                    let title = snippet.title.unwrap_or_default();
                    let description = snippet.description.unwrap_or_default();
                    let published_at = snippet.published_at.unwrap_or_default();
                    let channel_title = snippet
                        .video_owner_channel_title
                        .clone()
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
        Ok(Page::new(videos, next_page_token))
    }

    /// List the videos inside a specific playlist.
    pub async fn list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> Result<Vec<Video>> {
        let page = self.list_playlist_videos_page(playlist_id, max_results, None).await?;
        Ok(page.items)
    }

    /// Subscribe to a channel.
    pub async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        use google_youtube3::api::{ResourceId, Subscription as YtSub, SubscriptionSnippet};
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
        })
        .await?;
        Ok(())
    }

    /// Unsubscribe from a channel using its subscription ID.
    pub async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.subscriptions().delete(subscription_id).doit().await
        })
        .await?;
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
        })
        .await?;
        let id = playlist_res.id.unwrap_or_default();
        let title = playlist_res
            .snippet
            .as_ref()
            .and_then(|s| s.title.clone())
            .unwrap_or_default();
        let description = playlist_res
            .snippet
            .as_ref()
            .and_then(|s| s.description.clone())
            .unwrap_or_default();
        let thumbnail_url = playlist_res
            .snippet
            .as_ref()
            .and_then(|s| s.thumbnails.clone())
            .and_then(|t| t.default)
            .and_then(|t| t.url)
            .unwrap_or_default();
        let video_count = playlist_res
            .content_details
            .and_then(|cd| cd.item_count)
            .unwrap_or(0);

        Ok(Playlist {
            id,
            title,
            description,
            thumbnail_url,
            video_count,
        })
    }

    /// Delete a playlist owned by the authenticated user.
    pub async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.playlists().delete(playlist_id).doit().await
        })
        .await?;
        Ok(())
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
        })
        .await?;
        Ok(())
    }

    /// Remove a video from a playlist using its playlist item ID.
    pub async fn remove_from_playlist(&self, playlist_item_id: &str) -> Result<()> {
        retry_api_call(|| async {
            self.hub.playlist_items().delete(playlist_item_id).doit().await
        })
        .await?;
        Ok(())
    }

    /// Rate a video ("like", "dislike", or "none") using a raw string or [`Rating`] enum.
    pub async fn rate_video(&self, video_id: &str, rating: impl AsRef<str>) -> Result<()> {
        let rating_str = rating.as_ref();
        retry_api_call(|| async {
            self.hub.videos().rate(video_id, rating_str).doit().await
        })
        .await?;
        Ok(())
    }

    /// Rate a video with typed [`Rating`].
    pub async fn rate_video_typed(&self, video_id: &str, rating: Rating) -> Result<()> {
        self.rate_video(video_id, rating.as_api_str()).await
    }

    /// Fetch top comment threads for a video.
    pub async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        let (_resp, comment_res) = retry_api_call(|| async {
            self.hub
                .comment_threads()
                .list(&vec!["snippet".to_string()])
                .video_id(video_id)
                .max_results(20)
                .clear_scopes()
                .add_scope(google_youtube3::api::Scope::Readonly)
                .doit()
                .await
        })
        .await?;

        let mut comments = Vec::new();
        if let Some(items) = comment_res.items {
            for item in items {
                if let Some(snippet) = item
                    .snippet
                    .and_then(|s| s.top_level_comment)
                    .and_then(|c| c.snippet)
                {
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

    /// Post a new top-level comment on a video.
    pub async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment> {
        use google_youtube3::api::{
            Comment as YtComment, CommentSnippet as YtCommentSnippet,
            CommentThread, CommentThreadSnippet,
        };

        let comment_snippet = YtCommentSnippet {
            text_original: Some(text.to_string()),
            ..Default::default()
        };
        let top_level = YtComment {
            snippet: Some(comment_snippet),
            ..Default::default()
        };
        let thread_snippet = CommentThreadSnippet {
            video_id: Some(video_id.to_string()),
            top_level_comment: Some(top_level),
            ..Default::default()
        };
        let thread = CommentThread {
            snippet: Some(thread_snippet),
            ..Default::default()
        };

        let (_resp, created_thread) = retry_api_call(|| async {
            self.hub
                .comment_threads()
                .insert(thread.clone())
                .add_part("snippet")
                .doit()
                .await
        })
        .await?;

        let snippet = created_thread
            .snippet
            .and_then(|s| s.top_level_comment)
            .and_then(|c| c.snippet)
            .unwrap_or_default();

        Ok(Comment {
            author_name: snippet.author_display_name.unwrap_or_else(|| "You".to_string()),
            author_thumbnail: snippet.author_profile_image_url.unwrap_or_default(),
            text_display: snippet.text_display.unwrap_or_else(|| text.to_string()),
            published_at: snippet.published_at.map(|dt| dt.to_string()).unwrap_or_else(|| "Just now".to_string()),
            like_count: 0,
        })
    }

    /// Download a YouTube video by ID to the target path.
    pub async fn download_video<F>(&self, video_id: &str, output_path: &Path, on_progress: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        crate::download::download_video_direct(video_id, output_path, on_progress).await
    }

    /// Download a YouTube video by ID to the target path with custom [`DownloadOptions`].
    pub async fn download_video_with_options<F>(
        &self,
        video_id: &str,
        output_path: &Path,
        options: &crate::download::DownloadOptions,
        on_progress: F,
    ) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        crate::download::download_video_with_options(video_id, output_path, options, on_progress).await
    }

    /// Play the video using the system's default media player.
    pub fn play_video_system(&self, file_path: &Path) -> Result<()> {
        open::that(file_path).map_err(|e| YoutubeError::Media(e.to_string()))?;
        Ok(())
    }

    /// Play the audio of the downloaded video file using Rodio.
    pub async fn play_audio_rodio(file_path: std::path::PathBuf) -> Result<()> {
        crate::audio::play_audio_rodio(file_path).await
    }

    /// Test the connection to YouTube by querying a single subscription.
    pub async fn test_connection(&self) -> Result<()> {
        let _ = self.list_subscriptions(1).await?;
        Ok(())
    }
}

impl SubscriptionService for YoutubeClient {
    async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>> {
        self.list_subscriptions(max_results).await
    }
    async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()> {
        self.subscribe_to_channel(channel_id).await
    }
    async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()> {
        self.unsubscribe_from_channel(subscription_id).await
    }
}

impl VideoService for YoutubeClient {
    async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>> {
        self.list_videos(channel_id, max_results).await
    }
    async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>> {
        self.search_videos(query, max_results).await
    }
    async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()> {
        self.rate_video(video_id, rating).await
    }
    async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails> {
        self.fetch_video_details(video_id).await
    }
}

impl PlaylistService for YoutubeClient {
    async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>> {
        self.list_playlists(max_results).await
    }
    async fn list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> Result<Vec<Video>> {
        self.list_playlist_videos(playlist_id, max_results).await
    }
    async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()> {
        self.add_to_playlist(playlist_id, video_id).await
    }
    async fn create_playlist(&self, title: &str, description: Option<&str>) -> Result<Playlist> {
        self.create_playlist(title, description).await
    }
    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        self.delete_playlist(playlist_id).await
    }
}

impl CommentService for YoutubeClient {
    async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>> {
        self.fetch_comments(video_id).await
    }
    async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment> {
        self.post_comment(video_id, text).await
    }
}

impl MediaDownloader for YoutubeClient {
    async fn download_media(
        &self,
        video_id: &str,
        output_path: &Path,
        progress_cb: Box<dyn Fn(&str) + Send + Sync>,
    ) -> Result<()> {
        self.download_video(video_id, output_path, progress_cb).await
    }
}

/// Initialize a `YoutubeClient` with resolved credentials and standard YouTube scopes.
pub async fn init_client(
    opt_client_id: Option<String>,
    opt_client_secret: Option<String>,
    config_path: &Path,
    token_cache_path: &Path,
) -> Result<YoutubeClient> {
    let (client_id, client_secret) =
        resolve_credentials(opt_client_id, opt_client_secret, config_path)?;

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
