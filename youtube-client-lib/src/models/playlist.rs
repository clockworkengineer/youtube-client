#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub video_count: u32,
}
