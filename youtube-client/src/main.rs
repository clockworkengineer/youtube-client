//! # YouTube CLI Command-Line Application
//!
//! Command-line client interface supporting OAuth login, listing subscriptions/videos,
//! downloading media streams, and local audio playback.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

mod commands;

use commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => {
            execute_login(cli.client_id, cli.client_secret, &cli.config, &cli.token_cache).await
        }
        Commands::Subscriptions { limit } => {
            execute_subscriptions(cli.client_id, cli.client_secret, &cli.config, &cli.token_cache, limit).await
        }
        Commands::Videos { channel_id, limit } => {
            execute_videos(cli.client_id, cli.client_secret, &cli.config, &cli.token_cache, channel_id, limit).await
        }
        Commands::Download { video_id, output } => {
            execute_download(cli.client_id, cli.client_secret, &cli.config, &cli.token_cache, video_id, output).await
        }
        Commands::Play { file, system } => {
            execute_play(file, system).await
        }
    }
}

