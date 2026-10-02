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
