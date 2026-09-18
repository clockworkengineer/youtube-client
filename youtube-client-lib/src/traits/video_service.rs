use crate::Result;
use crate::models::{Video, VideoDetails};

pub trait VideoService: Send + Sync {
    async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>>;
    async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>>;
    async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()>;
    async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails>;
}
