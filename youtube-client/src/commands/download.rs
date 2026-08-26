use std::io::Write;
use std::path::PathBuf;
use youtube_client_lib::init_client;

pub async fn execute_download(
    client_id: Option<String>,
    client_secret: Option<String>,
    config: &PathBuf,
    token_cache: &PathBuf,
    video_id: String,
    output: PathBuf,
) -> anyhow::Result<()> {
    println!("Starting download for video {}...", video_id);
    let client_res = init_client(client_id, client_secret, config, token_cache).await;
    if client_res.is_err() {
        println!("No OAuth credentials provided (optional for download). Using direct downloader...");
    } else {
        println!("Downloading via client...");
    }

    youtube_client_lib::download_video_direct(&video_id, &output, |prog| {
        print!("\r{}", prog);
        let _ = std::io::stdout().flush();
    })
    .await?;
    println!("\nDownload complete! Saved to {:?}", output);
    Ok(())
}
