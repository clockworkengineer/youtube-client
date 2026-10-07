//! # Local Audio Playback Subsystem

use crate::error::{Result, YoutubeError};
use std::path::PathBuf;

/// Play audio from a local media file using Rodio.
#[cfg(feature = "audio")]
pub async fn play_audio_rodio(file_path: PathBuf) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        use rodio::{Decoder, DeviceSinkBuilder, Player};
        use std::fs::File;
        use std::io::BufReader;

        let handle = DeviceSinkBuilder::open_default_sink().map_err(|e| {
            YoutubeError::Media(format!("Failed to open default audio stream: {e:?}"))
        })?;
        let player = Player::connect_new(handle.mixer());

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)
            .map_err(|e| YoutubeError::Media(format!("Failed to decode audio: {e}")))?;

        player.append(source);
        player.play();
        player.sleep_until_end();
        Ok(())
    })
    .await
    .map_err(|e| YoutubeError::Other(format!("Audio thread panicked: {e}")))?
}

/// Fallback when the `audio` feature is disabled.
#[cfg(not(feature = "audio"))]
pub async fn play_audio_rodio(_file_path: PathBuf) -> Result<()> {
    Err(YoutubeError::Media(
        "The 'audio' feature is disabled in youtube-client-lib.".to_string(),
    ))
}

/// Stream audio directly from YouTube via `yt-dlp` piping without intermediate disk writes.
#[cfg(feature = "audio")]
pub async fn stream_audio_rodio(
    video_id: &str,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    use tokio::io::AsyncReadExt;

    let url = format!("https://www.youtube.com/watch?v={video_id}");
    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.arg("-f")
        .arg("bestaudio")
        .arg("-o")
        .arg("-")
        .arg("--")
        .arg(&url);

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => YoutubeError::YtDlpMissing(e.to_string()),
            _ => YoutubeError::Io(e),
        })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| YoutubeError::Media("Failed to capture yt-dlp stdout".to_string()))?;

    let mut audio_bytes = Vec::new();
    let mut temp = [0u8; 8192];
    let cancel_clone = cancellation_token.clone();

    loop {
        let n = tokio::select! {
            _ = async {
                if let Some(ref token) = cancel_clone {
                    token.cancelled().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                let _ = child.kill().await;
                return Err(YoutubeError::Cancelled);
            }
            res = stdout.read(&mut temp) => {
                res?
            }
        };

        if n == 0 {
            break;
        }
        audio_bytes.extend_from_slice(&temp[..n]);
    }

    let _ = child.wait().await;

    if audio_bytes.is_empty() {
        return Err(YoutubeError::Media("No audio data received from stream".to_string()));
    }

    tokio::task::spawn_blocking(move || {
        use rodio::{Decoder, DeviceSinkBuilder, Player};
        use std::io::Cursor;

        let handle = DeviceSinkBuilder::open_default_sink().map_err(|e| {
            YoutubeError::Media(format!("Failed to open default audio stream: {e:?}"))
        })?;
        let player = Player::connect_new(handle.mixer());

        let reader = Cursor::new(audio_bytes);
        let source = Decoder::new(reader)
            .map_err(|e| YoutubeError::Media(format!("Failed to decode audio stream: {e}")))?;

        player.append(source);
        player.play();

        while !player.empty() {
            if let Some(ref token) = cancellation_token {
                if token.is_cancelled() {
                    player.stop();
                    return Err(YoutubeError::Cancelled);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Ok(())
    })
    .await
    .map_err(|e| YoutubeError::Other(format!("Audio playback thread panicked: {e}")))?
}

/// Fallback when the `audio` feature is disabled.
#[cfg(not(feature = "audio"))]
pub async fn stream_audio_rodio(
    _video_id: &str,
    _cancellation_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    Err(YoutubeError::Media(
        "The 'audio' feature is disabled in youtube-client-lib.".to_string(),
    ))
}
