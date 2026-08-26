#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Subscription {
    pub id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub thumbnail_url: String,
}
