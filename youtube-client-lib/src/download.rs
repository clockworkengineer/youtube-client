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

/// Configuration options for downloading media.
#[derive(Clone, Debug)]
pub struct DownloadOptions {
    pub format: DownloadFormat,
    pub extractor_args: Option<String>,
    pub additional_args: Vec<String>,
    pub log_file: Option<std::path::PathBuf>,
    pub cookies_file: Option<std::path::PathBuf>,
    pub cookies_from_browser: Option<String>,
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
    F: Fn(&str) + Send + Sync + 'static,
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
    F: Fn(&str) + Send + Sync + 'static,
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
    F: Fn(&str) + Send + Sync + 'static,
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

    on_progress("Starting download...");

    loop {
        let n = stdout.read(&mut temp_buf).await?;
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
                    if line.contains("[download]") {
                        if let Some(pct_idx) = line.find('%') {
                            if let Some(dl_idx) = line.find("[download]") {
                                let start = dl_idx + 10;
                                if start < pct_idx {
                                    let pct = line[start..pct_idx].trim();
                                    on_progress(&format!("Downloading: {pct}%"));
                                }
                            }
                        } else if line.contains("Destination:") {
                            on_progress("Starting download...");
                        }
                    } else if line.contains("[ExtractAudio]") || line.contains("[ffmpeg]") {
                        on_progress("Extracting audio...");
                    }
                }
            }
            buffer.drain(..pos + 1);
        }
    }

    let status = child.wait().await?;
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
    F: Fn(&str) + Send + Sync + 'static,
{
    Err(YoutubeError::Download(
        "The 'download' feature is disabled in youtube-client-lib.".to_string(),
    ))
}
