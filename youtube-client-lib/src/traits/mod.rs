#![allow(async_fn_in_trait)]

pub mod comment_service;
pub mod downloader;
pub mod playlist_service;
pub mod subscription_service;
pub mod video_service;

pub use comment_service::CommentService;
pub use downloader::MediaDownloader;
pub use playlist_service::PlaylistService;
pub use subscription_service::SubscriptionService;
pub use video_service::VideoService;

/// Composite service trait combining all YouTube client domain capabilities.
pub trait YoutubeApiService:
    VideoService + SubscriptionService + PlaylistService + CommentService + Send + Sync
{
}

impl<T> YoutubeApiService for T where
    T: VideoService + SubscriptionService + PlaylistService + CommentService + Send + Sync
{
}
