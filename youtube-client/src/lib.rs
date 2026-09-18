//! # YouTube CLI Library Interface
//!
//! Provides CLI definition, subcommands, and execution dispatcher for `youtube-client`.

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub mod commands;
pub use commands::*;

#[derive(Parser, Debug)]
#[command(
    name = "youtube-client",
    about = "High-performance YouTube API v3 CLI client",
    long_about = "A versatile YouTube command-line interface for querying feeds, searching videos, managing playlists, rating media, and downloading audio/video streams.\n\nEXAMPLES:\n  # Search videos with table output\n  youtube-client search --query \"Rust async programming\" --limit 5\n\n  # Stream subscriptions as JSON for piping to jq\n  youtube-client subscriptions --all --json | jq '.[].title'\n\n  # Download audio stream as MP3\n  youtube-client download --video-id dQw4w9WgXcQ --format mp3\n\n  # Rate a video\n  youtube-client rate --video-id dQw4w9WgXcQ --rating like"
)]
pub struct Cli {
    /// Path to config file containing client credentials
    #[arg(short, long, default_value = "config.json")]
    pub config: PathBuf,

    /// Path to store the cached token
    #[arg(short, long, default_value = "tokencache.json")]
    pub token_cache: PathBuf,

    /// Google OAuth2 Client ID
    #[arg(long, env = "GOOGLE_CLIENT_ID")]
    pub client_id: Option<String>,

    /// Google OAuth2 Client Secret
    #[arg(long, env = "GOOGLE_CLIENT_SECRET")]
    pub client_secret: Option<String>,

    /// Path to client log file for subprocess and media trace output
    #[arg(short, long, env = "YOUTUBE_CLIENT_LOG_FILE")]
    pub log_file: Option<PathBuf>,

    /// Path to cookies.txt file for yt-dlp/MPV authentication
    #[arg(long, env = "YOUTUBE_COOKIES_FILE")]
    pub cookies: Option<PathBuf>,

    /// Extract cookies from browser (e.g. chrome, firefox, edge, brave)
    #[arg(long, env = "YOUTUBE_COOKIES_FROM_BROWSER")]
    pub cookies_from_browser: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    /// Authenticate with Google OAuth2 and save the token cache
    Login,

    /// List your YouTube subscriptions
    Subscriptions {
        /// Limit the number of subscriptions to return
        #[arg(short, long, default_value_t = 20)]
        limit: u32,

        /// Resumption page token for pagination
        #[arg(long)]
        page_token: Option<String>,

        /// Fetch all subscription pages exhaustively
        #[arg(long, default_value_t = false)]
        all: bool,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// List uploads/videos from a specific YouTube channel
    Videos {
        /// The YouTube Channel ID (e.g. UC_x5XG1OV2P6uZZ5FSM9Ttw)
        #[arg(short, long)]
        channel_id: String,

        /// Limit the number of videos to return
        #[arg(short, long, default_value_t = 20)]
        limit: u32,

        /// Resumption page token for pagination
        #[arg(long)]
        page_token: Option<String>,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Search for videos across YouTube
    Search {
        /// The search query
        #[arg(short, long)]
        query: String,

        /// Limit the number of results to return
        #[arg(short, long, default_value_t = 20)]
        limit: u32,

        /// Resumption page token for pagination
        #[arg(long)]
        page_token: Option<String>,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Rate a YouTube video (like, dislike, or remove rating)
    Rate {
        /// The YouTube Video ID
        #[arg(short, long)]
        video_id: String,

        /// The rating to apply
        #[arg(short, long, value_enum)]
        rating: CliRating,
    },

    /// List your playlists or retrieve items within a specific playlist
    Playlists {
        /// Limit the number of playlists or items to return
        #[arg(short, long, default_value_t = 25)]
        limit: u32,

        /// Optional Playlist ID to list video items from
        #[arg(short, long)]
        playlist_id: Option<String>,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Download a YouTube video by ID
    Download {
        /// The YouTube Video ID (e.g. dQw4w9WgXcQ)
        #[arg(short, long)]
        video_id: String,

        /// Output path for the downloaded media (default: `<video_id>.<ext>`)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Format to download: mp4, mp3, bestaudio, or custom format string
        #[arg(short, long, default_value = "mp4")]
        format: String,

        /// Video quality constraint (e.g. 1080p, 720p)
        #[arg(short, long)]
        quality: Option<String>,

        /// Additional arguments passed directly to yt-dlp
        #[arg(long = "arg", allow_hyphen_values = true)]
        additional_args: Vec<String>,
    },

    /// Play a downloaded media file locally
    Play {
        /// Path to the downloaded media file
        #[arg(short, long)]
        file: PathBuf,

        /// Open using the system default media player instead of decoding audio locally
        #[arg(short, long)]
        system: bool,
    },

    /// Fetch rich video details (views, likes, comments, duration, tags)
    Details {
        /// The YouTube Video ID
        #[arg(short, long)]
        video_id: String,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Fetch channel profile and statistics (subscribers, views, video counts)
    Channel {
        /// The YouTube Channel ID
        #[arg(short, long)]
        channel_id: String,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// List top comment threads on a video
    Comments {
        /// The YouTube Video ID
        #[arg(short, long)]
        video_id: String,

        /// Maximum comments to list
        #[arg(short, long, default_value_t = 20)]
        limit: u32,

        /// Output results as formatted JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Post a new comment to a YouTube video
    CommentPost {
        /// The YouTube Video ID
        #[arg(short, long)]
        video_id: String,

        /// The text of the comment to post
        #[arg(short, long)]
        text: String,
    },

    /// Subscribe to a YouTube channel
    Subscribe {
        /// The YouTube Channel ID to subscribe to
        #[arg(short, long)]
        channel_id: String,
    },

    /// Unsubscribe from a YouTube channel
    Unsubscribe {
        /// The subscription ID to remove
        #[arg(short, long)]
        subscription_id: String,
    },

    /// Create a new playlist
    PlaylistCreate {
        /// Title of the new playlist
        #[arg(short, long)]
        title: String,

        /// Optional description for the playlist
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete a playlist owned by your account
    PlaylistDelete {
        /// The playlist ID to delete
        #[arg(short, long)]
        playlist_id: String,
    },

    /// Generate shell auto-completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliRating {
    Like,
    Dislike,
    None,
}

impl From<CliRating> for youtube_client_lib::models::Rating {
    fn from(r: CliRating) -> Self {
        match r {
            CliRating::Like => youtube_client_lib::models::Rating::Like,
            CliRating::Dislike => youtube_client_lib::models::Rating::Dislike,
            CliRating::None => youtube_client_lib::models::Rating::None,
        }
    }
}

/// Execute the CLI command.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let ctx = CliContext {
        client_id: cli.client_id,
        client_secret: cli.client_secret,
        config: cli.config,
        token_cache: cli.token_cache,
        log_file: cli.log_file,
        cookies_file: cli.cookies,
        cookies_from_browser: cli.cookies_from_browser,
    };

    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "youtube-client", &mut std::io::stdout());
            Ok(())
        }
        Commands::Login => execute_login(&ctx).await,
        Commands::Subscriptions {
            limit,
            page_token,
            all,
            json,
        } => execute_subscriptions(&ctx, limit, page_token, all, json).await,
        Commands::Videos {
            channel_id,
            limit,
            page_token,
            json,
        } => execute_videos(&ctx, channel_id, limit, page_token, json).await,
        Commands::Search {
            query,
            limit,
            page_token,
            json,
        } => execute_search(&ctx, query, limit, page_token, json).await,
        Commands::Rate { video_id, rating } => execute_rate(&ctx, video_id, rating.into()).await,
        Commands::Playlists {
            limit,
            playlist_id,
            json,
        } => execute_playlists(&ctx, limit, playlist_id, json).await,
        Commands::Download {
            video_id,
            output,
            format,
            quality,
            additional_args,
        } => execute_download(&ctx, video_id, output, format, quality, additional_args).await,
        Commands::Play { file, system } => execute_play(file, system).await,
        Commands::Details { video_id, json } => execute_details(&ctx, video_id, json).await,
        Commands::Channel { channel_id, json } => execute_channel(&ctx, channel_id, json).await,
        Commands::Comments {
            video_id,
            limit,
            json,
        } => execute_comments(&ctx, video_id, limit, json).await,
        Commands::CommentPost { video_id, text } => {
            execute_comment_post(&ctx, video_id, text).await
        }
        Commands::Subscribe { channel_id } => execute_subscribe(&ctx, channel_id).await,
        Commands::Unsubscribe { subscription_id } => {
            execute_unsubscribe(&ctx, subscription_id).await
        }
        Commands::PlaylistCreate { title, description } => {
            execute_playlist_create(&ctx, title, description).await
        }
        Commands::PlaylistDelete { playlist_id } => {
            execute_playlist_delete(&ctx, playlist_id).await
        }
    }
}
