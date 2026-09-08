//! # Error Types for YouTube Client Suite

/// Central error enum representing failure modes across OAuth, Google YouTube API,
/// network transport, media decoding, subprocess downloading, and filesystem I/O.
#[derive(thiserror::Error, Debug)]
pub enum YoutubeError {
    #[error("Credentials error: {0}")]
    Credentials(String),

    #[error("Authentication error: {0}")]
    Auth(#[from] yup_oauth2::Error),

    #[error("API request failed: {0}")]
    Api(#[from] google_youtube3::Error),

    #[error("API Quota exceeded. Please check your Google Developer Console quota limits: {0}")]
    QuotaExceeded(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Media error: {0}")]
    Media(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("yt-dlp is missing. Please make sure yt-dlp is installed and in your PATH: {0}")]
    YtDlpMissing(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Convenience result alias using [`YoutubeError`].
pub type Result<T> = std::result::Result<T, YoutubeError>;
