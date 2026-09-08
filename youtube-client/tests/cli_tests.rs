use clap::{CommandFactory, Parser};
use youtube_client::{Cli, Commands, CliRating};

#[test]
fn test_cli_debug_assert() {
    // Verifies all clap flags, commands, and arguments have valid setup with no collisions
    Cli::command().debug_assert();
}

#[test]
fn test_search_command_parsing() {
    let args = vec!["youtube-client", "search", "--query", "rust async", "--limit", "15", "--json"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse search command");

    match cli.command {
        Commands::Search { query, limit, page_token, json } => {
            assert_eq!(query, "rust async");
            assert_eq!(limit, 15);
            assert_eq!(page_token, None);
            assert!(json);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_rate_command_parsing() {
    let args = vec!["youtube-client", "rate", "--video-id", "dQw4w9WgXcQ", "--rating", "like"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse rate command");

    match cli.command {
        Commands::Rate { video_id, rating } => {
            assert_eq!(video_id, "dQw4w9WgXcQ");
            assert_eq!(rating, CliRating::Like);
        }
        _ => panic!("Expected Rate command"),
    }
}

#[test]
fn test_download_command_parsing() {
    let args = vec![
        "youtube-client",
        "download",
        "--video-id",
        "abc123xyz",
        "--format",
        "mp3",
        "--quality",
        "1080p",
        "--arg",
        "--no-playlist",
    ];
    let cli = Cli::try_parse_from(args).expect("Failed to parse download command");

    match cli.command {
        Commands::Download { video_id, format, quality, additional_args, .. } => {
            assert_eq!(video_id, "abc123xyz");
            assert_eq!(format, "mp3");
            assert_eq!(quality, Some("1080p".to_string()));
            assert_eq!(additional_args, vec!["--no-playlist"]);
        }
        _ => panic!("Expected Download command"),
    }
}

#[test]
fn test_subscriptions_pagination_parsing() {
    let args = vec![
        "youtube-client",
        "subscriptions",
        "--all",
        "--page-token",
        "CDIQAA",
        "--json",
    ];
    let cli = Cli::try_parse_from(args).expect("Failed to parse subscriptions command");

    match cli.command {
        Commands::Subscriptions { all, page_token, json, .. } => {
            assert!(all);
            assert_eq!(page_token, Some("CDIQAA".to_string()));
            assert!(json);
        }
        _ => panic!("Expected Subscriptions command"),
    }
}

#[test]
fn test_playlists_command_parsing() {
    let args = vec!["youtube-client", "playlists", "--playlist-id", "PL123456", "--limit", "50"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse playlists command");

    match cli.command {
        Commands::Playlists { limit, playlist_id, json } => {
            assert_eq!(limit, 50);
            assert_eq!(playlist_id, Some("PL123456".to_string()));
            assert!(!json);
        }
        _ => panic!("Expected Playlists command"),
    }
}
