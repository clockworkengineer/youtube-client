#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub published_at: String,
    pub thumbnail_url: String,
    pub channel_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist_item_id: Option<String>,
}

impl Video {
    /// Create a standard Video instance without an associated playlist item ID.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        published_at: impl Into<String>,
        thumbnail_url: impl Into<String>,
        channel_title: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            published_at: published_at.into(),
            thumbnail_url: thumbnail_url.into(),
            channel_title: channel_title.into(),
            playlist_item_id: None,
        }
    }
}
