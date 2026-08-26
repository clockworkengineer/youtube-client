use std::path::PathBuf;
use youtube_client_lib::YoutubeClient;

pub async fn execute_play(file: PathBuf, system: bool) -> anyhow::Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("Error: File {:?} does not exist!", file));
    }
    if system {
        println!("Opening {:?} in system media player...", file);
        open::that(&file)?;
    } else {
        println!("Decoding and playing audio from {:?} via Rodio...", file);
        println!("Press Ctrl+C to stop playback.");
        YoutubeClient::play_audio_rodio(file.clone()).await?;
    }
    println!("Playback finished.");
    Ok(())
}
