use crate::Result;
use std::path::Path;

pub trait MediaDownloader: Send + Sync {
    async fn download_media(
        &self,
        video_id: &str,
        output_path: &Path,
        progress_cb: Box<dyn Fn(&str) + Send + Sync>,
    ) -> Result<()>;
}
