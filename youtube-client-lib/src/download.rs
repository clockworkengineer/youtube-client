//! # Media Downloader Engine (yt-dlp integration)

use std::path::Path;
use crate::error::{Result, YoutubeError};

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
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            format: DownloadFormat::Mp4,
            extractor_args: Some("youtube:player_client=mweb".to_string()),
            additional_args: Vec::new(),
        }
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
    download_video_with_options(video_id, output_path, &DownloadOptions::default(), on_progress).await
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

    let url = format!("https://www.youtube.com/watch?v={}", video_id);
    let is_mp3 = match &options.format {
        DownloadFormat::Mp3 => true,
        DownloadFormat::Mp4 => output_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mp3")),
        _ => false,
    };

    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.arg("--newline")
       .arg("--no-keep-video");

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
                cmd.arg("-f").arg("bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b");
            }
        }
    }

    for arg in &options.additional_args {
        cmd.arg(arg);
    }

    // Use explicit argument boundaries
    cmd.arg("-o").arg(output_path).arg("--").arg(&url);

    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(YoutubeError::YtDlpMissing(e.to_string()));
        }
        Err(e) => return Err(YoutubeError::Io(e)),
    };

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| YoutubeError::Download("Failed to capture stdout".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| YoutubeError::Download("Failed to capture stderr".to_string()))?;

    let stderr_handle = tokio::spawn(async move {
        let mut err_buf = Vec::new();
        let mut temp_err = [0u8; 1024];
        while let Ok(n) = stderr.read(&mut temp_err).await {
            if n == 0 {
                break;
            }
            err_buf.extend_from_slice(&temp_err[..n]);
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
                    if line.contains("[download]") {
                        if let Some(pct_idx) = line.find('%') {
                            if let Some(dl_idx) = line.find("[download]") {
                                let start = dl_idx + 10;
                                if start < pct_idx {
                                    let pct = line[start..pct_idx].trim();
                                    on_progress(&format!("Downloading: {}%", pct));
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
        return Err(YoutubeError::Download(format!(
            "yt-dlp download failed: {}",
            stderr_output.trim()
        )));
    }
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
