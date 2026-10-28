//! Search query filters, sorting orders, duration options, and content types for YouTube API v3 search.

/// Ordering criteria for YouTube search results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SearchOrder {
    /// Sort by search relevance (default).
    Relevance,
    /// Sort by upload date (newest first).
    Date,
    /// Sort by total view count (highest first).
    ViewCount,
    /// Sort by community rating (highest rated first).
    Rating,
    /// Sort alphabetically by title.
    Title,
}

impl SearchOrder {
    /// Convert enum to the YouTube Data API v3 query parameter string.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Date => "date",
            Self::ViewCount => "viewCount",
            Self::Rating => "rating",
            Self::Title => "title",
        }
    }

    /// Human-readable display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Relevance => "Relevance",
            Self::Date => "Upload Date",
            Self::ViewCount => "View Count",
            Self::Rating => "Rating",
            Self::Title => "Title",
        }
    }

    /// Parse loosely from string.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "date" | "upload-date" | "upload_date" => Self::Date,
            "viewcount" | "view-count" | "view_count" | "views" => Self::ViewCount,
            "rating" => Self::Rating,
            "title" => Self::Title,
            _ => Self::Relevance,
        }
    }
}

/// Video duration filter criteria for YouTube search results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DurationFilter {
    /// Any video length.
    Any,
    /// Short videos (< 4 minutes).
    Short,
    /// Medium length videos (4 to 20 minutes).
    Medium,
    /// Long videos (> 20 minutes).
    Long,
}

impl DurationFilter {
    /// Convert enum to the YouTube Data API v3 query parameter string.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
        }
    }

    /// Human-readable display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Any => "Any Duration",
            Self::Short => "Short (< 4 min)",
            Self::Medium => "Medium (4–20 min)",
            Self::Long => "Long (> 20 min)",
        }
    }

    /// Parse loosely from string.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "short" => Self::Short,
            "medium" => Self::Medium,
            "long" => Self::Long,
            _ => Self::Any,
        }
    }
}

/// Content type filter for search results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SearchType {
    /// Video uploads only.
    Video,
    /// Channels only.
    Channel,
    /// Playlists only.
    Playlist,
    /// All content types.
    All,
}

impl SearchType {
    /// Convert enum to the YouTube Data API v3 query parameter string.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Channel => "channel",
            Self::Playlist => "playlist",
            Self::All => "video,channel,playlist",
        }
    }

    /// Human-readable display label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Video => "Videos",
            Self::Channel => "Channels",
            Self::Playlist => "Playlists",
            Self::All => "All",
        }
    }

    /// Parse loosely from string.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "channel" | "channels" => Self::Channel,
            "playlist" | "playlists" => Self::Playlist,
            "all" => Self::All,
            _ => Self::Video,
        }
    }
}

/// Comprehensive search filter options for querying YouTube videos, channels, and playlists.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchFilter {
    /// Result sorting order.
    pub order: Option<SearchOrder>,
    /// Video duration constraint.
    pub duration: Option<DurationFilter>,
    /// Content type filter (video, channel, playlist).
    pub content_type: Option<SearchType>,
    /// RFC 3339 timestamp string (e.g. `2026-01-01T00:00:00Z`) filtering items published after this date.
    pub published_after: Option<String>,
}
