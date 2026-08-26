#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub published_at: String,
    pub thumbnail_url: String,
    pub channel_title: String,
}
