use crate::models::{Playlist, Video};
use crate::Result;

pub trait PlaylistService: Send + Sync {
    async fn list_playlists(&self, max_results: u32) -> Result<Vec<Playlist>>;
    async fn list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> Result<Vec<Video>>;
    async fn add_to_playlist(&self, playlist_id: &str, video_id: &str) -> Result<()>;
}
