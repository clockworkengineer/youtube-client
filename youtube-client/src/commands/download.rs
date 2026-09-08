use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use youtube_client_lib::download::{DownloadFormat, DownloadOptions};
use youtube_client_lib::download_video_with_options;

pub async fn execute_download(
    video_id: String,
    output: Option<PathBuf>,
    format_str: String,
    quality: Option<String>,
    additional_args: Vec<String>,
) -> anyhow::Result<()> {
    let format = match format_str.to_lowercase().as_str() {
        "mp3" => DownloadFormat::Mp3,
        "bestaudio" | "audio" => DownloadFormat::BestAudio,
        "mp4" | "video" => DownloadFormat::Mp4,
        custom => DownloadFormat::Custom(custom.to_string()),
    };

    let default_ext = match format {
        DownloadFormat::Mp3 => "mp3",
        DownloadFormat::BestAudio => "m4a",
        _ => "mp4",
    };

    let out_path = output.unwrap_or_else(|| PathBuf::from(format!("{}.{}", video_id, default_ext)));

    println!("Downloading video {}...", video_id);
    println!("  Format: {:?}", format);
    println!("  Target: {:?}", out_path);

    let mut options = DownloadOptions::new(format);
    if let Some(q) = quality {
        options = options.with_quality(q);
    }
    for arg in additional_args {
        options = options.with_arg(arg);
    }

    // Throttled terminal progress updates (at most every 100ms) to prevent stdout thrashing
    let last_print = Arc::new(AtomicU64::new(0));
    let start_instant = Instant::now();

    download_video_with_options(&video_id, &out_path, &options, move |prog| {
        let now_ms = start_instant.elapsed().as_millis() as u64;
        let prev = last_print.load(Ordering::Relaxed);
        if now_ms.saturating_sub(prev) >= 100 || prog.contains("100%") {
            last_print.store(now_ms, Ordering::Relaxed);
            print!("\r{}", prog);
            let _ = std::io::stdout().flush();
        }
    })
    .await?;

    println!("\n✓ Download completed successfully! Saved to {:?}", out_path);
    Ok(())
}
