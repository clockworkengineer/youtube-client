use clap::{Parser, Subcommand};
use std::path::PathBuf;
use youtube_client::YoutubeClient;

#[derive(Parser)]
#[command(name = "youtube-client")]
#[command(about = "A Rust CLI YouTube client demo", long_about = None)]
struct Cli {
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

    let get_credentials = || -> anyhow::Result<(String, String)> {
        let cid = cli.client_id.clone().or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok());
        let csec = cli.client_secret.clone().or_else(|| std::env::var("GOOGLE_CLIENT_SECRET").ok());

        match (cid, csec) {
            (Some(id), Some(secret)) => Ok((id, secret)),
            _ => Err(anyhow::anyhow!(
                "Error: Google Client ID and Client Secret must be provided!\n\n\
                Please set the GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables,\n\
                or pass them using the --client-id and --client-secret command line arguments.\n\n\
                To get Google API Client credentials:\n\
                1. Go to the Google Cloud Console: https://console.cloud.google.com/\n\
                2. Create a project and search for the \"YouTube Data API v3\" and enable it.\n\
                3. Navigate to \"APIs & Services\" > \"Credentials\".\n\
                4. Click \"Create Credentials\" > \"OAuth client ID\". Choose \"Desktop app\".\n\
                5. Retrieve your Client ID and Client Secret."
            )),
        }
    };

    match cli.command {
        Commands::Login => {
            let (client_id, client_secret) = get_credentials()?;
            println!("Starting OAuth2 Login flow...");
            let _client = YoutubeClient::new_oauth(&client_id, &client_secret, &cli.token_cache).await?;
            println!("Login successful! Token saved to {:?}", cli.token_cache);
        }
        Commands::Subscriptions { limit } => {
            let (client_id, client_secret) = get_credentials()?;
            let client = YoutubeClient::new_oauth(&client_id, &client_secret, &cli.token_cache).await?;
            println!("Fetching subscriptions...");
            let subs = client.list_subscriptions(limit).await?;
            if subs.is_empty() {
                println!("No subscriptions found.");
            } else {
                println!("{:<5} {:<30} {:<30}", "Index", "Title", "Channel ID");
                println!("{}", "-".repeat(70));
                for (idx, sub) in subs.iter().enumerate() {
                    println!("{:<5} {:<30} {:<30}", idx + 1, truncate(&sub.title, 28), sub.channel_id);
                }
            }
        }
        Commands::Videos { channel_id, limit } => {
            let (client_id, client_secret) = get_credentials()?;
            let client = YoutubeClient::new_oauth(&client_id, &client_secret, &cli.token_cache).await?;
            println!("Fetching videos for channel {}...", channel_id);
            let videos = client.list_videos(&channel_id, limit).await?;
            if videos.is_empty() {
                println!("No videos found.");
            } else {
                println!("{:<5} {:<40} {:<15} {:<15}", "Index", "Title", "Video ID", "Published At");
                println!("{}", "-".repeat(80));
                for (idx, vid) in videos.iter().enumerate() {
                    println!(
                        "{:<5} {:<40} {:<15} {:<15}",
                        idx + 1,
                        truncate(&vid.title, 38),
                        vid.id,
                        truncate(&vid.published_at, 10)
                    );
                }
            }
        }
        Commands::Download { video_id, output } => {
            // Downloading public videos doesn't require Google Client credentials in rusty_ytdl
            println!("Starting download for video {}...", video_id);
            // Create dummy client just for downloading (doesn't need auth credentials)
            // But we can download it directly via the YoutubeClient library method.
            // Since `download_video` is a method on YoutubeClient, let's run it.
            // Note: because `download_video` doesn't use the YouTube API hub, we can build a simple dummy struct or call it.
            // Let's create an unauthenticated method or just instantiate a client or call it.
            // Since download_video is on YoutubeClient, we can make a dummy instance or make it a static/associated function if we want, or just log in.
            // Wait, we can authenticate or use credentials if they are provided, or we can add a method or make a dummy client.
            // Let's construct a dummy or get credentials. Since download uses standard HTTP/rusty_ytdl, we don't need real google keys.
            // But to make it easy, we'll try to get credentials if available, otherwise use a placeholder client ID/secret to construct client.
            let client = if let Ok((id, secret)) = get_credentials() {
                YoutubeClient::new_oauth(&id, &secret, &cli.token_cache).await?
            } else {
                // If credentials are not present, we can just instantiate a client with dummy keys since we only want to download.
                // But wait! YoutubeClient constructor calls yup-oauth2 which triggers authentication!
                // To avoid requiring authentication for just downloading or playing, let's expose these as static/associated functions
                // or have an unauthenticated way.
                // In our lib, download_video does not use `self.hub`, so we can make it an associated function!
                // Or we can just call it on a dummy client. Let's make it associated. Let's check `lib.rs`.
                // Actually, `download_video` takes `&self` but doesn't use it. Let's make it `pub async fn download_video(video_id: &str, output_path: &Path) -> anyhow::Result<()>`!
                // And same for `play_audio_rodio` and `play_video_system`.
                // This is a great design! Let's modify the library to make them associated functions so they can be run without OAuth login.
                println!("No OAuth credentials provided (optional for download). Using direct downloader...");
                let url = format!("https://www.youtube.com/watch?v={}", video_id);
                let video = rusty_ytdl::Video::new(url)?;
                video.download(&output).await?;
                println!("Download complete! Saved to {:?}", output);
                return Ok(());
            };
            println!("Downloading via client...");
            client.download_video(&video_id, &output).await?;
            println!("Download complete! Saved to {:?}", output);
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
                // Use a direct rodio player since we don't need OAuth
                use rodio::{Decoder, DeviceSinkBuilder, Player};
                use std::fs::File;
                use std::io::BufReader;

                let handle = DeviceSinkBuilder::open_default_sink()
                    .map_err(|e| anyhow::anyhow!("Failed to open default audio stream: {:?}", e))?;
                let player = Player::connect_new(&handle.mixer());
                let f = File::open(&file)?;
                let reader = BufReader::new(f);
                let source = Decoder::new(reader)
                    .map_err(|e| anyhow::anyhow!("Failed to decode audio: {}", e))?;
                player.append(source);
                player.sleep_until_end();
            }
            println!("Playback finished.");
        }
    }

    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
}
