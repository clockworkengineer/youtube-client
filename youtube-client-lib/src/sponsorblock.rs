//! # SponsorBlock Integration
//!
//! Provides integration with the public SponsorBlock API to retrieve community-submitted
//! segment timestamps for skipping sponsors, intros, outros, and self-promotions.

use crate::error::{Result, YoutubeError};
use serde::{Deserialize, Serialize};

/// Categories of segments tracked by SponsorBlock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentCategory {
    Sponsor,
    Selfpromo,
    Interaction,
    Intro,
    Outro,
    Preview,
    MusicOfftopic,
    #[serde(other)]
    Other,
}

impl SegmentCategory {
    /// Return the category identifier as used in the SponsorBlock API.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Sponsor => "sponsor",
            Self::Selfpromo => "selfpromo",
            Self::Interaction => "interaction",
            Self::Intro => "intro",
            Self::Outro => "outro",
            Self::Preview => "preview",
            Self::MusicOfftopic => "music_offtopic",
            Self::Other => "other",
        }
    }
}

/// A skip segment timestamp range with categorization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipSegment {
    pub category: SegmentCategory,
    pub action_type: String,
    pub start_secs: f64,
    pub end_secs: f64,
    pub uuid: String,
}

#[derive(Deserialize)]
struct RawSegmentResponse {
    category: SegmentCategory,
    #[serde(rename = "actionType")]
    action_type: String,
    segment: (f64, f64),
    #[serde(rename = "UUID")]
    uuid: String,
}

/// Default categories requested for skipping.
pub const DEFAULT_CATEGORIES: &[&str] = &[
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro",
];

/// Fetch skip segments for a video from the SponsorBlock API.
///
/// If the video has no submitted segments (HTTP 404), an empty list is returned.
pub async fn fetch_skip_segments(
    http_client: &reqwest::Client,
    video_id: &str,
    categories: Option<&[&str]>,
) -> Result<Vec<SkipSegment>> {
    let cats = categories.unwrap_or(DEFAULT_CATEGORIES);
    let cats_json = serde_json::to_string(&cats)
        .map_err(|e| YoutubeError::Other(format!("Failed to serialize categories: {e}")))?;

    let url = format!(
        "https://sponsor.ajay.app/api/skipSegments?videoID={video_id}&categories={cats_json}"
    );

    let resp = match http_client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return Err(YoutubeError::Other(format!("SponsorBlock query failed: {e}"))),
    };

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let raw_list: Vec<RawSegmentResponse> = match resp.json().await {
        Ok(l) => l,
        Err(_) => return Ok(Vec::new()),
    };

    let segments = raw_list
        .into_iter()
        .map(|r| SkipSegment {
            category: r.category,
            action_type: r.action_type,
            start_secs: r.segment.0,
            end_secs: r.segment.1,
            uuid: r.uuid,
        })
        .collect();

    Ok(segments)
}

/// Generate an MPV Edit Decision List (EDL) specification to skip specified segments seamlessly.
///
/// If `skip_segments` is empty or no valid cuts can be made, returns `None`.
pub fn build_mpv_edl(media_url: &str, total_duration_secs: f64, skip_segments: &[SkipSegment]) -> Option<String> {
    if skip_segments.is_empty() || total_duration_secs <= 0.0 {
        return None;
    }

    // Sort segments by start time
    let mut sorted = skip_segments.to_vec();
    sorted.sort_by(|a, b| a.start_secs.partial_cmp(&b.start_secs).unwrap());

    let mut parts: Vec<(f64, f64)> = Vec::new();
    let mut current_pos = 0.0;

    for seg in &sorted {
        if seg.start_secs > current_pos {
            let duration = seg.start_secs - current_pos;
            if duration > 0.1 {
                parts.push((current_pos, duration));
            }
        }
        if seg.end_secs > current_pos {
            current_pos = seg.end_secs;
        }
    }

    if current_pos < total_duration_secs {
        let remaining = total_duration_secs - current_pos;
        if remaining > 0.1 {
            parts.push((current_pos, remaining));
        }
    }

    if parts.is_empty() {
        return None;
    }

    // Format: edl://%length%media_url,start,length;...
    let mut edl = String::from("edl://");
    for (idx, (start, length)) in parts.iter().enumerate() {
        if idx > 0 {
            edl.push(';');
        }
        edl.push_str(&format!("%{}%{},{:.2},{:.2}", media_url.len(), media_url, start, length));
    }

    Some(edl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mpv_edl() {
        let segments = vec![
            SkipSegment {
                category: SegmentCategory::Intro,
                action_type: "skip".to_string(),
                start_secs: 0.0,
                end_secs: 10.0,
                uuid: "seg1".to_string(),
            },
            SkipSegment {
                category: SegmentCategory::Sponsor,
                action_type: "skip".to_string(),
                start_secs: 50.0,
                end_secs: 80.0,
                uuid: "seg2".to_string(),
            },
        ];

        let edl = build_mpv_edl("https://example.com/video", 100.0, &segments).unwrap();
        assert!(edl.starts_with("edl://"));
        // First cut: 10.0 to 50.0 (duration 40.0)
        assert!(edl.contains(",10.00,40.00"));
        // Second cut: 80.0 to 100.0 (duration 20.0)
        assert!(edl.contains(",80.00,20.00"));
    }
}
