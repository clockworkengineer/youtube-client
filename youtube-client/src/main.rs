use clap::{Parser, Subcommand};
use std::path::PathBuf;
use youtube_client_lib::YoutubeClient;

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


    let get_credentials = || -> anyhow::Result<(String, String)> {
        let mut cid = cli.client_id.clone().or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok());
        let mut csec = cli.client_secret.clone().or_else(|| std::env::var("GOOGLE_CLIENT_SECRET").ok());

        // If credentials are still missing, try loading from the config file
        if cid.is_none() || csec.is_none() {
            let config = if cli.config != std::path::PathBuf::from("config.json") {
                if let Ok(file_content) = std::fs::read_to_string(&cli.config) {
                    serde_json::from_str::<youtube_client_lib::Config>(&file_content).unwrap_or_default()
                } else {
                    youtube_client_lib::load_config()
                }
            } else {
                youtube_client_lib::load_config()
            };

            if cid.is_none() {
                cid = config.client_id;
            }
            if csec.is_none() {
                csec = config.client_secret;
            }
        }

        let temp_config = youtube_client_lib::Config {
            client_id: cid,
            client_secret: csec,
            player_path: None,
            downloads_dir: None,
        };

        if temp_config.is_valid() {
            Ok((temp_config.client_id.unwrap(), temp_config.client_secret.unwrap()))
        } else {
            Err(anyhow::anyhow!(
                "Error: Google Client ID and Client Secret must be provided!\n\n{}",
                youtube_client_lib::GOOGLE_SETUP_INSTRUCTIONS
            ))
        }
    };
    let get_client = || async {
        let (client_id, client_secret) = get_credentials()?;
        YoutubeClient::new_oauth_with_scopes(
            &client_id,
            &client_secret,
            &cli.token_cache,
            &[
                "https://www.googleapis.com/auth/youtube",
                "https://www.googleapis.com/auth/youtube.force-ssl",
                "https://www.googleapis.com/auth/youtube.readonly"
            ],
        ).await
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
                |sub, idx| vec![(idx + 1).to_string(), truncate(&sub.title, 28), sub.channel_id.clone()],
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
                    truncate(&vid.title, 38),
                    vid.id.clone(),
                    truncate(&vid.published_at, 10),
                ],
            );
        }
        Commands::Download { video_id, output } => {
            println!("Starting download for video {}...", video_id);
            let client = match get_client().await {
                Ok(c) => c,
                Err(_) => {
                    println!("No OAuth credentials provided (optional for download). Using direct downloader...");
                    let url = format!("https://www.youtube.com/watch?v={}", video_id);
                    let mut cmd = tokio::process::Command::new("yt-dlp");
                    let is_mp3 = output.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mp3"));
                    if is_mp3 {
                        cmd.arg("-x").arg("--audio-format").arg("mp3");
                    } else {
                        cmd.arg("-f").arg("bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]");
                    }
                    cmd.arg("-o").arg(&output).arg(&url);
                    let status = cmd.status().await?;
                    if !status.success() {
                        anyhow::bail!("yt-dlp download failed");
                    }
                    println!("Download complete! Saved to {:?}", output);
                    return Ok(());
                }
            };
            println!("Downloading via client...");
            client.download_video(&video_id, &output, |prog| {
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

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
    }
}

fn print_table<T>(
    headers: &[&str],
    widths: &[usize],
    items: &[T],
    row_formatter: impl Fn(&T, usize) -> Vec<String>,
) {
    if items.is_empty() {
        println!("No items found.");
        return;
    }

    for (i, header) in headers.iter().enumerate() {
        print!("{:<width$} ", header, width = widths[i]);
    }
    println!();

    let total_width: usize = widths.iter().sum::<usize>() + widths.len() - 1;
    println!("{}", "-".repeat(total_width));

    for (idx, item) in items.iter().enumerate() {
        let cols = row_formatter(item, idx);
        for (i, col) in cols.iter().enumerate() {
            print!("{:<width$} ", col, width = widths[i]);
        }
        println!();
    }
}
