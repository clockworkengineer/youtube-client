//! # Strongly-Typed Video Rating Enum

use std::str::FromStr;

/// Video rating option for YouTube videos.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rating {
    #[serde(rename = "like")]
    Like,
    #[serde(rename = "dislike")]
    Dislike,
    #[serde(rename = "none")]
    None,
}

impl Rating {
    /// Return the string representation required by the YouTube Data API v3.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Rating::Like => "like",
            Rating::Dislike => "dislike",
            Rating::None => "none",
        }
    }
}

impl std::fmt::Display for Rating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_api_str())
    }
}

impl AsRef<str> for Rating {
    fn as_ref(&self) -> &str {
        self.as_api_str()
    }
}

impl FromStr for Rating {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "like" => Ok(Rating::Like),
            "dislike" => Ok(Rating::Dislike),
            "none" => Ok(Rating::None),
            other => Err(format!(
                "Invalid rating '{other}', expected 'like', 'dislike', or 'none'"
            )),
        }
    }
}
