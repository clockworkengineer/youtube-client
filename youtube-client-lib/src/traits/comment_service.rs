use crate::Result;
use crate::models::{Comment, Page};

pub trait CommentService: Send + Sync {
    async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>>;
    async fn fetch_comments_page(
        &self,
        video_id: &str,
        max_results: u32,
        page_token: Option<&str>,
    ) -> Result<Page<Comment>> {
        let _ = (max_results, page_token);
        let items = self.fetch_comments(video_id).await?;
        Ok(Page::new(items, None))
    }
    async fn post_comment(&self, video_id: &str, text: &str) -> Result<Comment>;
}
