use crate::Result;
use crate::models::{SearchFilter, Video, VideoDetails};

pub trait VideoService: Send + Sync {
    async fn list_videos(&self, channel_id: &str, max_results: u32) -> Result<Vec<Video>>;
    async fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<Video>>;
    async fn search_videos_with_filter(
        &self,
        query: &str,
        max_results: u32,
        filter: Option<&SearchFilter>,
    ) -> Result<Vec<Video>> {
        let _ = filter;
        self.search_videos(query, max_results).await
    }
    async fn rate_video(&self, video_id: &str, rating: &str) -> Result<()>;
    async fn fetch_video_details(&self, video_id: &str) -> Result<VideoDetails>;
    async fn get_videos_batch(&self, video_ids: &[&str]) -> Result<Vec<VideoDetails>>;
}
