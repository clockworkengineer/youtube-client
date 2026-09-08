//! # Detailed Video Information & Statistics Model

use serde::{Deserialize, Serialize};

/// Extended video details including engagement metrics, duration, and metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoDetails {
    /// Unique YouTube video identifier
    pub id: String,
    /// Video title
    pub title: String,
    /// Video description
    pub description: String,
    /// Publication timestamp (ISO 8601)
    pub published_at: String,
    /// Channel identifier
    pub channel_id: String,
    /// Channel title / creator name
    pub channel_title: String,
    /// URL of the highest resolution thumbnail available
    pub thumbnail_url: String,
    /// Total view count
    pub view_count: u64,
    /// Total like count
    pub like_count: u64,
    /// Total top-level comment count
    pub comment_count: u64,
    /// Duration in seconds
    pub duration_seconds: u64,
    /// Formatted duration string (e.g. "12:34" or "1:02:45")
    pub duration_formatted: String,
    /// Topic and content tags
    pub tags: Vec<String>,
}

impl VideoDetails {
    /// Parse an ISO 8601 duration string (e.g. `PT1H2M10S`, `PT4M33S`, `PT45S`) into seconds.
    pub fn parse_iso8601_duration(duration: &str) -> u64 {
        let mut total_secs: u64 = 0;
        let mut current_num: u64 = 0;

        let trimmed = duration.trim_start_matches('P').trim_start_matches('T');
        for ch in trimmed.chars() {
            if ch.is_ascii_digit() {
                if let Some(digit) = ch.to_digit(10) {
                    current_num = current_num * 10 + digit as u64;
                }
            } else {
                match ch {
                    'H' => {
                        total_secs += current_num * 3600;
                        current_num = 0;
                    }
                    'M' => {
                        total_secs += current_num * 60;
                        current_num = 0;
                    }
                    'S' => {
                        total_secs += current_num;
                        current_num = 0;
                    }
                    _ => {}
                }
            }
        }
        total_secs
    }

    /// Format total seconds into a readable timestamp string (e.g. `12:34` or `1:02:45`).
    pub fn format_duration(total_seconds: u64) -> String {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}", minutes, seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601_duration() {
        assert_eq!(VideoDetails::parse_iso8601_duration("PT4M33S"), 273);
        assert_eq!(VideoDetails::parse_iso8601_duration("PT1H2M10S"), 3730);
        assert_eq!(VideoDetails::parse_iso8601_duration("PT45S"), 45);
        assert_eq!(VideoDetails::parse_iso8601_duration("PT1H"), 3600);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(VideoDetails::format_duration(273), "4:33");
        assert_eq!(VideoDetails::format_duration(3730), "1:02:10");
        assert_eq!(VideoDetails::format_duration(45), "0:45");
    }
}
