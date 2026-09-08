//! # Detailed Channel Profile & Statistics Model

use serde::{Deserialize, Serialize};

/// Extended channel details including subscriber counts, video totals, and branding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelDetails {
    /// Unique YouTube channel identifier (e.g. `UC_x5XG1OV2P6uZZ5FSM9Ttw`)
    pub id: String,
    /// Channel display name
    pub title: String,
    /// Channel description / bio
    pub description: String,
    /// Custom handle or vanity URL (e.g. `@GoogleDevelopers`)
    pub custom_url: Option<String>,
    /// Highest resolution avatar thumbnail URL
    pub thumbnail_url: String,
    /// Total subscriber count
    pub subscriber_count: u64,
    /// Total public videos uploaded
    pub video_count: u64,
    /// Total lifetime view count
    pub view_count: u64,
}
