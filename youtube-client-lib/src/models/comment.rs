#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Comment {
    pub author_name: String,
    pub author_thumbnail: String,
    pub text_display: String,
    pub published_at: String,
    pub like_count: u32,
}
