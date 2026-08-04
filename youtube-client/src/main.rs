//! # YouTube CLI Command-Line Application
//!
//! Command-line client interface supporting OAuth login, listing subscriptions/videos,
//! downloading media streams, and local audio playback.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use youtube_client_lib::utils::{print_table, truncate};
use youtube_client_lib::{init_client, YoutubeClient};

#[derive(Parser)]
#[command(name = "youtube-client")]
#[command(about = "A Rust CLI YouTube client demo", long_about = None)]
struct Cli {
    /// Path to config file containing client credentials
    #[arg(short, long, default_value = "config.json")]
    config: std::path::PathBuf,

    /// Path to store the cached token
    #[arg(short, long, default_value = "tokencache.json")]
    token_cache: PathBuf,

    /// Google OAuth2 Client ID
    #[arg(long, env = "GOOGLE_CLIENT_ID")]
    client_id: Option<String>,

    /// Google OAuth2 Client Secret
    #[arg(long, env = "GOOGLE_CLIENT_SECRET")]
    client_secret: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Google OAuth2 and save the token cache
    Login,
    /// List your YouTube subscriptions
    Subscriptions {
        /// Limit the number of subscriptions to list
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
    },
    /// List uploads/videos from a YouTube channel
    Videos {
        /// The YouTube Channel ID
        #[arg(short, long)]
        channel_id: String,
        /// Limit the number of videos to list
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
    },
    /// Download a YouTube video by its video ID
    Download {
        /// The YouTube Video ID (e.g. dQw4w9WgXcQ)
        #[arg(short, long)]
        video_id: String,
        /// Output path for the downloaded video
        #[arg(short, long, default_value = "video.mp4")]
        output: PathBuf,
    },
    /// Play a downloaded video/audio file
    Play {
        /// Path to the downloaded media file
        #[arg(short, long)]
        file: PathBuf,
        /// Use system default media player instead of decoding audio locally
        #[arg(short, long)]
        system: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let get_client = || async {
        init_client(
            cli.client_id.clone(),
            cli.client_secret.clone(),
            &cli.config,
            &cli.token_cache,
        )
        .await
    };

    match cli.command {
        Commands::Login => {
            println!("Starting OAuth2 Login flow with full YouTube permissions...");
            let _client = get_client().await?;
            println!("Login successful! Token saved to {:?}", cli.token_cache);
        }
        Commands::Subscriptions { limit } => {
            let client = get_client().await?;
            println!("Fetching subscriptions...");
            let subs = client.list_subscriptions(limit).await?;
            print_table(
                &["Index", "Title", "Channel ID"],
                &[5, 30, 30],
                &subs,
                |sub, idx| vec![(idx + 1).to_string(), truncate(&sub.title, 28).into_owned(), sub.channel_id.clone()],
            );
        }
        Commands::Videos { channel_id, limit } => {
            let client = get_client().await?;
            println!("Fetching videos for channel {}...", channel_id);
            let videos = client.list_videos(&channel_id, limit).await?;
            print_table(
                &["Index", "Title", "Video ID", "Published At"],
                &[5, 40, 15, 15],
                &videos,
                |vid, idx| vec![
                    (idx + 1).to_string(),
                    truncate(&vid.title, 38).into_owned(),
                    vid.id.clone(),
                    truncate(&vid.published_at, 10).into_owned(),
                ],
            );
        }
        Commands::Download { video_id, output } => {
            println!("Starting download for video {}...", video_id);
            let client_res = get_client().await;
            if client_res.is_err() {
                println!("No OAuth credentials provided (optional for download). Using direct downloader...");
            } else {
                println!("Downloading via client...");
            }

            youtube_client_lib::download_video_direct(&video_id, &output, |prog| {
                print!("\r{}", prog);
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }).await?;
            println!("\nDownload complete! Saved to {:?}", output);
        }
        Commands::Play { file, system } => {
            if !file.exists() {
                return Err(anyhow::anyhow!("Error: File {:?} does not exist!", file));
            }
            if system {
                println!("Opening {:?} in system media player...", file);
                open::that(&file)?;
            } else {
                println!("Decoding and playing audio from {:?} via Rodio...", file);
                println!("Press Ctrl+C to stop playback.");
                YoutubeClient::play_audio_rodio(&file)?;
            }
            println!("Playback finished.");
        }
    }

    Ok(())
}

