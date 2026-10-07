//! # Media Downloader Engine (yt-dlp integration)

use crate::error::{Result, YoutubeError};
use std::path::Path;

/// Desired output media format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadFormat {
    /// MP4 video container with best video and audio streams merged.
    Mp4,
    /// Extracted MP3 audio stream.
    Mp3,
    /// Best quality raw audio.
    BestAudio,
    /// Custom yt-dlp format selector string.
    Custom(String),
}

/// Stage of the media download process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DownloadStage {
    /// Preparing the download / querying metadata.
    Starting,
    /// Downloading video / audio streams.
    Downloading,
    /// Merging streams or extracting audio via ffmpeg.
    ExtractingAudio,
    /// Download completed successfully.
    Finished,
}

impl std::fmt::Display for DownloadStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStage::Starting => write!(f, "Starting"),
            DownloadStage::Downloading => write!(f, "Downloading"),
            DownloadStage::ExtractingAudio => write!(f, "Extracting audio"),
            DownloadStage::Finished => write!(f, "Finished"),
        }
    }
}

/// Structured download progress snapshot reported during `yt-dlp` operations.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DownloadProgress {
    /// Current download stage.
    pub stage: DownloadStage,
    /// Completion percentage (0.0 to 100.0).
    pub percent: Option<f32>,
    /// Downloaded data in bytes.
    pub downloaded_bytes: Option<u64>,
    /// Total expected size in bytes.
    pub total_bytes: Option<u64>,
    /// Download speed in bytes per second.
    pub speed_bytes_per_sec: Option<u64>,
    /// Estimated time remaining in seconds.
    pub eta_seconds: Option<u64>,
    /// Formatted status line.
    pub raw_message: String,
}

impl std::fmt::Display for DownloadProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_message)
    }
}

impl std::ops::Deref for DownloadProgress {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.raw_message
    }
}

impl DownloadProgress {
    /// Construct a progress snapshot for the initial startup phase.
    pub fn starting(msg: impl Into<String>) -> Self {
        let raw = msg.into();
        Self {
            stage: DownloadStage::Starting,
            percent: None,
            downloaded_bytes: None,
            total_bytes: None,
            speed_bytes_per_sec: None,
            eta_seconds: None,
            raw_message: raw,
        }
    }

    /// Construct a progress snapshot for audio/post-processing extraction.
    pub fn extracting(msg: impl Into<String>) -> Self {
        let raw = msg.into();
        Self {
            stage: DownloadStage::ExtractingAudio,
            percent: None,
            downloaded_bytes: None,
            total_bytes: None,
            speed_bytes_per_sec: None,
            eta_seconds: None,
            raw_message: raw,
        }
    }

    /// Construct a progress snapshot for download completion.
    pub fn finished(msg: impl Into<String>) -> Self {
        let raw = msg.into();
        Self {
            stage: DownloadStage::Finished,
            percent: Some(100.0),
            downloaded_bytes: None,
            total_bytes: None,
            speed_bytes_per_sec: None,
            eta_seconds: Some(0),
            raw_message: raw,
        }
    }

    /// Parse a raw stdout line from `yt-dlp` into a structured progress snapshot.
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.contains("[ExtractAudio]") || trimmed.contains("[ffmpeg]") {
            return Some(Self::extracting("Extracting audio..."));
        }

        if !trimmed.contains("[download]") {
            return None;
        }

        if trimmed.contains("Destination:") {
            return Some(Self::starting("Starting download..."));
        }

        if let Some(pct_pos) = trimmed.find('%') {
            if let Some(dl_idx) = trimmed.find("[download]") {
                let start = dl_idx + 10;
                if start < pct_pos {
                    let pct_str = trimmed[start..pct_pos].trim();
                    let percent = pct_str.parse::<f32>().ok();

                    let mut total_bytes = None;
                    let mut speed_bytes_per_sec = None;
                    let mut eta_seconds = None;

                    let after_pct = &trimmed[pct_pos + 1..];
                    if let Some(of_idx) = after_pct.find("of") {
                        let after_of = &after_pct[of_idx + 2..];
                        let parts: Vec<&str> = after_of.split_whitespace().collect();
                        for (i, &token) in parts.iter().enumerate() {
                            let clean_token = token.trim_start_matches('~');
                            if let Some(bytes) = parse_byte_size(clean_token) {
                                total_bytes = Some(bytes);
                            } else if clean_token.chars().all(|c| c.is_ascii_digit() || c == '.') {
                                if let Some(&unit) = parts.get(i + 1) {
                                    if let Some(bytes) = parse_byte_size(&format!("{clean_token}{unit}")) {
                                        total_bytes = Some(bytes);
                                    }
                                }
                            }
                            if total_bytes.is_some() {
                                break;
                            }
                        }
                    }

                    if let Some(at_idx) = after_pct.find("at") {
                        let after_at = &after_pct[at_idx + 2..];
                        if let Some(token) = after_at.split_whitespace().next() {
                            let clean_token = token.trim_end_matches("/s");
                            speed_bytes_per_sec = parse_byte_size(clean_token);
                        }
                    }

                    if let Some(eta_idx) = after_pct.find("ETA") {
                        let after_eta = &after_pct[eta_idx + 3..];
                        if let Some(token) = after_eta.split_whitespace().next() {
                            eta_seconds = parse_eta_seconds(token);
                        }
                    }

                    let downloaded_bytes = match (percent, total_bytes) {
                        (Some(p), Some(tot)) => Some(((p as f64 / 100.0) * tot as f64) as u64),
                        _ => None,
                    };

                    let stage = if percent.is_some_and(|p| p >= 100.0) {
                        DownloadStage::Finished
                    } else {
                        DownloadStage::Downloading
                    };

                    let raw_message = if let Some(p) = percent {
                        format!("Downloading: {p}%")
                    } else {
                        format!("Downloading: {pct_str}%")
                    };

                    return Some(Self {
                        stage,
                        percent,
                        downloaded_bytes,
                        total_bytes,
                        speed_bytes_per_sec,
                        eta_seconds,
                        raw_message,
                    });
                }
            }
        }

        None
    }
}

fn parse_byte_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num_part, multiplier) = if let Some(stripped) = s.strip_suffix("GiB") {
        (stripped, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(stripped) = s.strip_suffix("GB") {
        (stripped, 1000.0 * 1000.0 * 1000.0)
    } else if let Some(stripped) = s.strip_suffix("MiB") {
        (stripped, 1024.0 * 1024.0)
    } else if let Some(stripped) = s.strip_suffix("MB") {
        (stripped, 1000.0 * 1000.0)
    } else if let Some(stripped) = s.strip_suffix("KiB") {
        (stripped, 1024.0)
    } else if let Some(stripped) = s.strip_suffix("KB") {
        (stripped, 1000.0)
    } else {
        let stripped = s.strip_suffix('B')?;
        (stripped, 1.0)
    };
    num_part.trim().parse::<f64>().ok().map(|v| (v * multiplier) as u64)
}

fn parse_eta_seconds(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    match parts.len() {
        2 => {
            let mins = parts[0].parse::<u64>().ok()?;
            let secs = parts[1].parse::<u64>().ok()?;
            Some(mins * 60 + secs)
        }
        3 => {
            let hours = parts[0].parse::<u64>().ok()?;
            let mins = parts[1].parse::<u64>().ok()?;
            let secs = parts[2].parse::<u64>().ok()?;
            Some(hours * 3600 + mins * 60 + secs)
        }
        _ => None,
    }
}

/// Configuration options for downloading media.
#[derive(Clone, Debug)]
pub struct DownloadOptions {
    pub format: DownloadFormat,
    pub extractor_args: Option<String>,
    pub additional_args: Vec<String>,
    pub log_file: Option<std::path::PathBuf>,
    pub cookies_file: Option<std::path::PathBuf>,
    pub cookies_from_browser: Option<String>,
    pub cancellation_token: Option<tokio_util::sync::CancellationToken>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            format: DownloadFormat::Mp4,
            extractor_args: Some("youtube:player_client=mweb".to_string()),
            additional_args: Vec::new(),
            log_file: None,
            cookies_file: None,
            cookies_from_browser: None,
            cancellation_token: None,
        }
    }
}

impl DownloadOptions {
    /// Create options with a specific format.
    pub fn new(format: DownloadFormat) -> Self {
        Self {
            format,
            ..Default::default()
        }
    }

    /// Add a quality constraint (e.g. "1080p", "720p").
    pub fn with_quality(mut self, quality: impl Into<String>) -> Self {
        let q = quality.into();
        self.additional_args.push("-S".to_string());
        self.additional_args
            .push(format!("res:{}", q.trim_end_matches('p')));
        self
    }

    /// Add an additional argument for yt-dlp.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.additional_args.push(arg.into());
        self
    }

    /// Set a custom client log file to capture all output and ffmpeg traces.
    pub fn with_log_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.log_file = Some(path.into());
        self
    }

    /// Set path to cookies file for authenticated yt-dlp downloading.
    pub fn with_cookies_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.cookies_file = Some(path.into());
        self
    }

    /// Set browser name to extract cookies from.
    pub fn with_cookies_from_browser(mut self, browser: impl Into<String>) -> Self {
        self.cookies_from_browser = Some(browser.into());
        self
    }

    /// Set a cooperative cancellation token.
    pub fn with_cancellation_token(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }
}

/// Download a YouTube video directly by ID using `yt-dlp`.
///
/// Features:
/// - Real-time progress parsing via stdout callback
/// - Supports custom quality/format options
/// - Protected by positional argument boundaries (`--`) against command-line flag injection
#[cfg(feature = "download")]
pub async fn download_video_direct<F>(
    video_id: &str,
    output_path: &Path,
    on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    download_video_with_options(
        video_id,
        output_path,
        &DownloadOptions::default(),
        on_progress,
    )
    .await
}

#[cfg(not(feature = "download"))]
pub async fn download_video_direct<F>(
    _video_id: &str,
    _output_path: &Path,
    _on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    Err(YoutubeError::Download(
        "The 'download' feature is disabled in youtube-client-lib.".to_string(),
    ))
}

/// Download a YouTube video using `yt-dlp` with explicit [`DownloadOptions`].
#[cfg(feature = "download")]
pub async fn download_video_with_options<F>(
    video_id: &str,
    output_path: &Path,
    options: &DownloadOptions,
    on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    use tokio::io::AsyncReadExt;

    let url = format!("https://www.youtube.com/watch?v={video_id}");
    let is_mp3 = match &options.format {
        DownloadFormat::Mp3 => true,
        DownloadFormat::Mp4 => output_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp3")),
        _ => false,
    };

    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.arg("--newline").arg("--no-keep-video");

    if let Some(ref ext_args) = options.extractor_args {
        cmd.arg("--extractor-args").arg(ext_args);
    }

    match &options.format {
        DownloadFormat::Mp3 => {
            cmd.arg("-x").arg("--audio-format").arg("mp3");
        }
        DownloadFormat::BestAudio => {
            cmd.arg("-x");
        }
        DownloadFormat::Custom(fmt) => {
            cmd.arg("-f").arg(fmt);
        }
        DownloadFormat::Mp4 => {
            if is_mp3 {
                cmd.arg("-x").arg("--audio-format").arg("mp3");
            } else {
                cmd.arg("-f")
                    .arg("bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b");
            }
        }
    }

    for arg in &options.additional_args {
        cmd.arg(arg);
    }

    let resolved_cookies_file = options
        .cookies_file
        .clone()
        .or_else(|| crate::config::resolve_cookies_file(None));
    if let Some(ref cf) = resolved_cookies_file {
        cmd.arg("--cookies").arg(cf);
    }

    let resolved_cookies_browser = options
        .cookies_from_browser
        .clone()
        .or_else(|| crate::config::resolve_cookies_from_browser(None));
    if let Some(ref cb) = resolved_cookies_browser {
        cmd.arg("--cookies-from-browser").arg(cb);
    }

    // Use explicit argument boundaries
    cmd.arg("-o").arg(output_path).arg("--").arg(&url);

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW to suppress ffmpeg and yt-dlp console windows

    let log_file_path = options
        .log_file
        .clone()
        .unwrap_or_else(|| crate::config::resolve_log_file_path(None));

    crate::utils::append_to_log(
        &log_file_path,
        "INFO",
        &format!(
            "Starting yt-dlp download: video_id={}, output={:?}, format={:?}",
            video_id, output_path, options.format
        ),
    );

    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            crate::utils::append_to_log(&log_file_path, "ERROR", &format!("yt-dlp missing: {e}"));
            return Err(YoutubeError::YtDlpMissing(e.to_string()));
        }
        Err(e) => {
            crate::utils::append_to_log(
                &log_file_path,
                "ERROR",
                &format!("Failed to spawn yt-dlp: {e}"),
            );
            return Err(YoutubeError::Io(e));
        }
    };

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| YoutubeError::Download("Failed to capture stdout".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| YoutubeError::Download("Failed to capture stderr".to_string()))?;

    let log_path_clone = log_file_path.clone();
    let stderr_handle = tokio::spawn(async move {
        let mut err_buf = Vec::new();
        let mut temp_err = [0u8; 1024];
        let mut line_buf = Vec::new();
        while let Ok(n) = stderr.read(&mut temp_err).await {
            if n == 0 {
                break;
            }
            err_buf.extend_from_slice(&temp_err[..n]);
            line_buf.extend_from_slice(&temp_err[..n]);
            while let Some(pos) = line_buf.iter().position(|&b| b == b'\n' || b == b'\r') {
                let line_bytes = &line_buf[..pos];
                if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                    let trimmed = line_str.trim();
                    if !trimmed.is_empty() {
                        crate::utils::append_to_log(&log_path_clone, "STDERR", trimmed);
                    }
                }
                line_buf.drain(..pos + 1);
            }
        }
        if !line_buf.is_empty() {
            if let Ok(line_str) = std::str::from_utf8(&line_buf) {
                let trimmed = line_str.trim();
                if !trimmed.is_empty() {
                    crate::utils::append_to_log(&log_path_clone, "STDERR", trimmed);
                }
            }
        }
        String::from_utf8_lossy(&err_buf).into_owned()
    });

    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 1024];
    let cancellation_token = options.cancellation_token.clone();

    on_progress(DownloadProgress::starting("Starting download..."));

    loop {
        let n = tokio::select! {
            _ = async {
                if let Some(ref token) = cancellation_token {
                    token.cancelled().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                let _ = child.kill().await;
                crate::utils::append_to_log(&log_file_path, "WARN", "Download cancelled by token");
                return Err(YoutubeError::Cancelled);
            }
            read_res = stdout.read(&mut temp_buf) => {
                read_res?
            }
        };

        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp_buf[..n]);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n' || b == b'\r') {
            let line_bytes = &buffer[..pos];
            if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                let line = line_str.trim();
                if !line.is_empty() {
                    crate::utils::append_to_log(&log_file_path, "STDOUT", line);
                    if let Some(progress) = DownloadProgress::parse_line(line) {
                        on_progress(progress);
                    }
                }
            }
            buffer.drain(..pos + 1);
        }
    }

    let status = tokio::select! {
        _ = async {
            if let Some(ref token) = cancellation_token {
                token.cancelled().await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            let _ = child.kill().await;
            crate::utils::append_to_log(&log_file_path, "WARN", "Download cancelled while awaiting process");
            return Err(YoutubeError::Cancelled);
        }
        wait_res = child.wait() => {
            wait_res?
        }
    };

    let stderr_output = stderr_handle.await.unwrap_or_default();
    if !status.success() {
        crate::utils::append_to_log(
            &log_file_path,
            "ERROR",
            &format!(
                "yt-dlp download failed with status {:?}: {}",
                status.code(),
                stderr_output.trim()
            ),
        );
        return Err(YoutubeError::Download(format!(
            "yt-dlp download failed: {}",
            stderr_output.trim()
        )));
    }
    crate::utils::append_to_log(
        &log_file_path,
        "INFO",
        &format!("Download finished successfully: {output_path:?}"),
    );
    on_progress(DownloadProgress::finished("Download complete"));
    Ok(())
}

#[cfg(not(feature = "download"))]
pub async fn download_video_with_options<F>(
    _video_id: &str,
    _output_path: &Path,
    _options: &DownloadOptions,
    _on_progress: F,
) -> Result<()>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    Err(YoutubeError::Download(
        "The 'download' feature is disabled in youtube-client-lib.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_byte_size() {
        assert_eq!(parse_byte_size("1024B"), Some(1024));
        assert_eq!(parse_byte_size("10KiB"), Some(10240));
        assert_eq!(parse_byte_size("5MiB"), Some(5 * 1024 * 1024));
        assert_eq!(parse_byte_size("1GiB"), Some(1024 * 1024 * 1024));
    }

    #[test]
    fn test_parse_eta_seconds() {
        assert_eq!(parse_eta_seconds("00:45"), Some(45));
        assert_eq!(parse_eta_seconds("01:23"), Some(83));
        assert_eq!(parse_eta_seconds("01:02:03"), Some(3600 + 120 + 3));
    }

    #[test]
    fn test_download_progress_parsing() {
        let line = "[download]  45.2% of ~ 50.00MiB at 10.20MiB/s ETA 01:23";
        let prog = DownloadProgress::parse_line(line).expect("should parse");
        assert_eq!(prog.stage, DownloadStage::Downloading);
        assert_eq!(prog.percent, Some(45.2));
        assert!(prog.total_bytes.is_some());
        assert!(prog.speed_bytes_per_sec.is_some());
        assert_eq!(prog.eta_seconds, Some(83));
        assert_eq!(prog.raw_message, "Downloading: 45.2%");

        let dest_line = "[download] Destination: video.mp4";
        let dest_prog = DownloadProgress::parse_line(dest_line).expect("should parse destination");
        assert_eq!(dest_prog.stage, DownloadStage::Starting);

        let audio_line = "[ExtractAudio] Destination: song.mp3";
        let audio_prog = DownloadProgress::parse_line(audio_line).expect("should parse extract");
        assert_eq!(audio_prog.stage, DownloadStage::ExtractingAudio);
    }
}
