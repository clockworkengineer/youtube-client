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
