use std::path::PathBuf;
use youtube_client_lib::YoutubeClient;

pub async fn execute_play(file: PathBuf, system: bool) -> anyhow::Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("File {:?} does not exist!", file));
    }

    if system {
        println!("Attempting to open {:?} in system media player...", file);
        match open::that(&file) {
            Ok(_) => {
                println!("Media launched in external player.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Warning: Failed to launch system player ({:?}). Falling back to local audio playback...", e);
            }
        }
    }

    println!("Playing audio from {:?} via Rodio...", file);
    println!("Press Ctrl+C to stop playback.");
    YoutubeClient::play_audio_rodio(file.clone()).await?;
    println!("Playback finished.");
    Ok(())
}
