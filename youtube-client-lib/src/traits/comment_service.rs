use crate::models::Comment;
use crate::Result;

pub trait CommentService: Send + Sync {
    async fn fetch_comments(&self, video_id: &str) -> Result<Vec<Comment>>;
}
