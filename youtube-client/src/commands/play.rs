use std::path::PathBuf;
use youtube_client_lib::YoutubeClient;

pub async fn execute_play(file: PathBuf, system: bool) -> anyhow::Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("File {file:?} does not exist!"));
    }

    if system {
        println!("Attempting to open {file:?} in system media player...");
        match open::that(&file) {
            Ok(_) => {
                println!("Media launched in external player.");
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to launch system player ({e:?}). Falling back to local audio playback..."
                );
            }
        }
    }

    println!("Playing audio from {file:?} via Rodio...");
    println!("Press Ctrl+C to stop playback.");
    YoutubeClient::play_audio_rodio(file.clone()).await?;
    println!("Playback finished.");
    Ok(())
}
