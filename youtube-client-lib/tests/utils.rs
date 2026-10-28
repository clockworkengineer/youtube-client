//! Tests for shared utility functions

use std::collections::HashMap;
use std::path::PathBuf;
use youtube_client_lib::utils::{
    DownloadStatus, extract_video_id_from_path, sanitize_filename, scan_downloads_dir,
};

#[test]
fn test_extract_video_id_from_path() {
    let path = PathBuf::from("/tmp/abcdefghijk.mp4");
    assert_eq!(
        extract_video_id_from_path(&path),
        Some("abcdefghijk".to_string())
    );

    let path2 = PathBuf::from("/tmp/video_[abcdefghijk].mp4");
    assert_eq!(
        extract_video_id_from_path(&path2),
        Some("abcdefghijk".to_string())
    );

    let bad = PathBuf::from("/tmp/notvideo.txt");
    assert_eq!(extract_video_id_from_path(&bad), None);
}

#[test]
fn test_sanitize_filename() {
    assert_eq!(
        sanitize_filename("Hello/World?"),
        "Hello_World_".to_string()
    );
    assert_eq!(sanitize_filename("dots... "), "dots".to_string());
    let long = "A".repeat(100);
    let sanitized = sanitize_filename(&long);
    assert!(sanitized.len() <= 60);
}

#[test]
fn test_scan_downloads_dir() {
    use tempfile::tempdir;
    let tmp = tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("song.mp3"), b"data").unwrap();
    std::fs::write(dir.join("video.mp4"), b"data").unwrap();
    let mut map: HashMap<String, DownloadStatus> = HashMap::new();
    scan_downloads_dir(dir, &mut map);
    assert!(map.contains_key("song"));
    assert!(map.contains_key("video"));
}

#[test]
fn test_get_download_path() {
    use youtube_client_lib::utils::get_download_path;
    let base = std::path::Path::new("downloads");
    let path = get_download_path(base, "Channel Title?", "Video Title*", "abcdefghijk", false);
    assert_eq!(
        path,
        std::path::PathBuf::from("downloads/Channel Title_/Video Title_ [abcdefghijk].mp4")
    );
}

#[test]
fn test_string_set_persistence() {
    use std::collections::HashSet;
    use tempfile::tempdir;
    use youtube_client_lib::utils::{load_string_set_from_file, save_string_set_to_file};

    let tmp = tempdir().unwrap();
    let file_path = tmp.path().join("test_set.json");

    let mut original = HashSet::new();
    original.insert("video1".to_string());
    original.insert("video2".to_string());

    assert!(save_string_set_to_file(&file_path, &original).is_ok());
    let loaded = load_string_set_from_file(&file_path);
    assert_eq!(original, loaded);
}

#[test]
fn test_ryd_response_parsing() {
    let raw_json = r#"{"id":"dQw4w9WgXcQ","dateCreated":"2021-12-05T00:00:00Z","likes":16000000,"dislikes":420000,"rating":4.87,"viewCount":1500000000}"#;
    #[derive(serde::Deserialize)]
    struct RydResponse {
        dislikes: Option<u64>,
    }
    let parsed: Result<RydResponse, _> = serde_json::from_str(raw_json);
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().dislikes, Some(420_000));
}

#[test]
fn test_playback_positions_persistence() {
    use tempfile::tempdir;
    use youtube_client_lib::utils::{
        PlaybackProgress, load_playback_positions_from_file, save_playback_positions_to_file,
    };

    let tmp = tempdir().unwrap();
    let file_path = tmp.path().join("playback_positions.json");

    let mut map = HashMap::new();
    map.insert(
        "dQw4w9WgXcQ".to_string(),
        PlaybackProgress {
            position_secs: 142.5,
            duration_secs: 212.0,
            updated_at: 1720000000,
        },
    );

    assert!(save_playback_positions_to_file(&file_path, &map).is_ok());
    let loaded = load_playback_positions_from_file(&file_path);
    assert_eq!(loaded, map);
}

#[test]
fn test_format_table() {
    use youtube_client_lib::utils::format_table;

    let items = vec![("Item A", 10), ("Item B", 25)];
    let table = format_table(
        &["Name", "Count"],
        &[10, 8],
        &items,
        |(name, count), _idx| vec![name.to_string(), count.to_string()],
    );

    assert!(table.contains("Name"));
    assert!(table.contains("Count"));
    assert!(table.contains("Item A"));
    assert!(table.contains("Item B"));
    assert!(table.contains("10"));
    assert!(table.contains("25"));

    let empty: Vec<(&str, i32)> = vec![];
    let empty_table = format_table(&["Name"], &[10], &empty, |(n, _), _| vec![n.to_string()]);
    assert_eq!(empty_table, "No items found.");
}

#[test]
fn test_scan_downloaded_items() {
    use tempfile::tempdir;
    use youtube_client_lib::utils::scan_downloaded_items;

    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let sub = root.join("Channel Alpha");
    std::fs::create_dir_all(&sub).unwrap();

    let mp3_path = sub.join("Song Title [11111111111].mp3");
    let mp4_path = root.join("Video Title [22222222222].mp4");
    let ignore_path = root.join("notes.txt");

    std::fs::write(&mp3_path, b"dummy audio data 1234").unwrap();
    std::fs::write(&mp4_path, b"dummy video data 12345678").unwrap();
    std::fs::write(&ignore_path, b"not media").unwrap();

    let scanned = scan_downloaded_items(root);
    assert_eq!(scanned.len(), 2);

    let audio = scanned
        .iter()
        .find(|i| i.is_audio)
        .expect("Audio item missing");
    assert_eq!(audio.video_id, Some("11111111111".to_string()));
    assert_eq!(audio.size_bytes, 21);

    let video = scanned
        .iter()
        .find(|i| !i.is_audio)
        .expect("Video item missing");
    assert_eq!(video.video_id, Some("22222222222".to_string()));
    assert_eq!(video.size_bytes, 25);
}

#[test]
fn test_watch_history_helpers() {
    use tempfile::tempdir;
    use youtube_client_lib::utils::{
        PlaybackProgress, clear_watch_history, get_watch_history, remove_watch_history_entry,
        save_playback_positions_to_file,
    };

    let tmp = tempdir().unwrap();
    let file = tmp.path().join("playback_positions.json");

    let mut map = std::collections::HashMap::new();
    map.insert(
        "vid_old".to_string(),
        PlaybackProgress {
            position_secs: 10.0,
            duration_secs: 100.0,
            updated_at: 1000,
        },
    );
    map.insert(
        "vid_new".to_string(),
        PlaybackProgress {
            position_secs: 50.0,
            duration_secs: 100.0,
            updated_at: 2000,
        },
    );

    save_playback_positions_to_file(&file, &map).unwrap();

    let history = get_watch_history(&file);
    assert_eq!(history.len(), 2);
    // Should be sorted updated_at descending
    assert_eq!(history[0].video_id, "vid_new");
    assert_eq!(history[1].video_id, "vid_old");

    // Remove single entry
    remove_watch_history_entry(&file, "vid_new").unwrap();
    let history_after_remove = get_watch_history(&file);
    assert_eq!(history_after_remove.len(), 1);
    assert_eq!(history_after_remove[0].video_id, "vid_old");

    // Clear history
    clear_watch_history(&file).unwrap();
    let history_cleared = get_watch_history(&file);
    assert!(history_cleared.is_empty());
}

#[test]
fn test_validate_video_and_channel_ids_and_sanitize() {
    use youtube_client_lib::utils::{is_valid_channel_id, is_valid_video_id, sanitize_identifier};

    assert!(is_valid_video_id("dQw4w9WgXcQ"));
    assert!(is_valid_video_id("vid_123-ABC"));
    assert!(!is_valid_video_id(""));
    assert!(!is_valid_video_id("-flag_injection"));
    assert!(!is_valid_video_id("../evil/path"));
    assert!(!is_valid_video_id("id?extra=1&foo=bar"));

    assert!(is_valid_channel_id("UC_x5XG1OV2P6uZZ5FSM9Ttw"));
    assert!(is_valid_channel_id("@rustlang"));
    assert!(!is_valid_channel_id(""));
    assert!(!is_valid_channel_id("-flag"));
    assert!(!is_valid_channel_id("../../etc/passwd"));

    assert_eq!(sanitize_identifier("../../evil/vid?123"), "evilvid123");
    assert_eq!(sanitize_identifier("---"), "default_id");
}

#[test]
fn test_format_table_mismatched_widths() {
    use youtube_client_lib::utils::format_table;

    let items = vec![("Alpha", 1, "Extra")];
    // Fewer widths than headers and columns — must not panic
    let table = format_table(&["Col1", "Col2", "Col3"], &[5], &items, |(a, b, c), _| {
        vec![a.to_string(), b.to_string(), c.to_string()]
    });
    assert!(table.contains("Col1"));
    assert!(table.contains("Col3"));
    assert!(table.contains("Alpha"));
    assert!(table.contains("Extra"));

    // Empty widths slice — must not panic
    let table_empty_widths = format_table(&["Col1", "Col2"], &[], &items, |(a, b, _), _| {
        vec![a.to_string(), b.to_string()]
    });
    assert!(table_empty_widths.contains("Alpha"));
}

#[test]
fn test_scan_downloads_depth_bound() {
    use tempfile::tempdir;
    use youtube_client_lib::utils::{MAX_SCAN_DEPTH, scan_downloaded_items, scan_downloads_dir};

    let tmp = tempdir().unwrap();
    let mut current = tmp.path().to_path_buf();

    // Create a file at root (depth 0)
    std::fs::write(current.join("root_vid [11111111111].mp4"), b"root").unwrap();

    // Create nested directories up to MAX_SCAN_DEPTH + 2
    for d in 1..=(MAX_SCAN_DEPTH + 2) {
        current = current.join(format!("level_{d}"));
        std::fs::create_dir_all(&current).unwrap();
        let file_name = format!("vid_d{d} [{d:011}].mp4");
        std::fs::write(current.join(file_name), b"nested").unwrap();
    }

    let mut map = HashMap::new();
    scan_downloads_dir(tmp.path(), &mut map);
    let items = scan_downloaded_items(tmp.path());

    // Depth 0 ..= MAX_SCAN_DEPTH should be found (MAX_SCAN_DEPTH + 1 files total)
    assert_eq!(map.len(), MAX_SCAN_DEPTH + 1);
    assert_eq!(items.len(), MAX_SCAN_DEPTH + 1);

    // Deeper than MAX_SCAN_DEPTH must be ignored
    let too_deep_id = format!("{:011}", MAX_SCAN_DEPTH + 1);
    assert!(!map.contains_key(&too_deep_id));
}
