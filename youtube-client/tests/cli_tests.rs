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

#[test]
fn test_details_command_parsing() {
    let args = vec!["youtube-client", "details", "--video-id", "dQw4w9WgXcQ", "--json"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse details command");

    match cli.command {
        Commands::Details { video_id, json } => {
            assert_eq!(video_id, "dQw4w9WgXcQ");
            assert!(json);
        }
        _ => panic!("Expected Details command"),
    }
}

#[test]
fn test_channel_command_parsing() {
    let args = vec!["youtube-client", "channel", "--channel-id", "UC_x5XG1OV2P6uZZ5FSM9Ttw"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse channel command");

    match cli.command {
        Commands::Channel { channel_id, json } => {
            assert_eq!(channel_id, "UC_x5XG1OV2P6uZZ5FSM9Ttw");
            assert!(!json);
        }
        _ => panic!("Expected Channel command"),
    }
}

#[test]
fn test_comments_and_post_command_parsing() {
    let args = vec!["youtube-client", "comments", "--video-id", "dQw4w9WgXcQ", "--limit", "30", "--json"];
    let cli = Cli::try_parse_from(args).expect("Failed to parse comments command");

    match cli.command {
        Commands::Comments { video_id, limit, json } => {
            assert_eq!(video_id, "dQw4w9WgXcQ");
            assert_eq!(limit, 30);
            assert!(json);
        }
        _ => panic!("Expected Comments command"),
    }

    let post_args = vec!["youtube-client", "comment-post", "--video-id", "dQw4w9WgXcQ", "--text", "Awesome video!"];
    let post_cli = Cli::try_parse_from(post_args).expect("Failed to parse comment-post command");

    match post_cli.command {
        Commands::CommentPost { video_id, text } => {
            assert_eq!(video_id, "dQw4w9WgXcQ");
            assert_eq!(text, "Awesome video!");
        }
        _ => panic!("Expected CommentPost command"),
    }
}

#[test]
fn test_subscription_manage_command_parsing() {
    let sub_args = vec!["youtube-client", "subscribe", "--channel-id", "UC12345"];
    let sub_cli = Cli::try_parse_from(sub_args).expect("Failed to parse subscribe command");
    match sub_cli.command {
        Commands::Subscribe { channel_id } => assert_eq!(channel_id, "UC12345"),
        _ => panic!("Expected Subscribe command"),
    }

    let unsub_args = vec!["youtube-client", "unsubscribe", "--subscription-id", "SUB_999"];
    let unsub_cli = Cli::try_parse_from(unsub_args).expect("Failed to parse unsubscribe command");
    match unsub_cli.command {
        Commands::Unsubscribe { subscription_id } => assert_eq!(subscription_id, "SUB_999"),
        _ => panic!("Expected Unsubscribe command"),
    }
}

#[test]
fn test_playlist_manage_command_parsing() {
    let create_args = vec![
        "youtube-client",
        "playlist-create",
        "--title",
        "Favorites 2026",
        "--description",
        "Best songs",
    ];
    let create_cli = Cli::try_parse_from(create_args).expect("Failed to parse playlist-create command");
    match create_cli.command {
        Commands::PlaylistCreate { title, description } => {
            assert_eq!(title, "Favorites 2026");
            assert_eq!(description, Some("Best songs".to_string()));
        }
        _ => panic!("Expected PlaylistCreate command"),
    }

    let delete_args = vec!["youtube-client", "playlist-delete", "--playlist-id", "PL_DEL_123"];
    let delete_cli = Cli::try_parse_from(delete_args).expect("Failed to parse playlist-delete command");
    match delete_cli.command {
        Commands::PlaylistDelete { playlist_id } => assert_eq!(playlist_id, "PL_DEL_123"),
        _ => panic!("Expected PlaylistDelete command"),
    }
}
