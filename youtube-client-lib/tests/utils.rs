//! Tests for shared utility functions

use youtube_client_lib::utils::{extract_video_id_from_path, sanitize_filename, scan_downloads_dir, DownloadStatus};
use std::path::PathBuf;
use std::collections::HashMap;

#[test]
fn test_extract_video_id_from_path() {
    let path = PathBuf::from("/tmp/abcdefghijk.mp4");
    assert_eq!(extract_video_id_from_path(&path), Some("abcdefghijk".to_string()));

    let path2 = PathBuf::from("/tmp/video_[abcdefghijk].mp4");
    assert_eq!(extract_video_id_from_path(&path2), Some("abcdefghijk".to_string()));

    let bad = PathBuf::from("/tmp/notvideo.txt");
    assert_eq!(extract_video_id_from_path(&bad), None);
}

#[test]
fn test_sanitize_filename() {
    assert_eq!(sanitize_filename("Hello/World?"), "Hello_World_".to_string());
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
