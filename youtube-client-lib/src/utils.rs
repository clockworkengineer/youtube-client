// Utility functions shared across crates

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents the download state of a video.
#[derive(Clone)]
pub enum DownloadStatus {
    NotStarted,
    Downloading { progress: String },
    Finished(PathBuf),
    Failed(String),
}


/// Extract a YouTube video ID from a file path (expects a stem of length 11).
pub fn extract_video_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.len() == 11 {
        return Some(stem.to_string());
    }
    // Look for pattern "[VIDEOID]" at the end of the filename
    let open_bracket = stem.rfind('[')?;
    let close_bracket = stem.rfind(']')?;
    if open_bracket < close_bracket && close_bracket == stem.len() - 1 {
        let id = &stem[open_bracket + 1..close_bracket];
        if id.len() == 11 {
            return Some(id.to_string());
        }
    }
    None
}

/// Sanitize a filename for safe filesystem usage.
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();

    // Truncate to a safe length (e.g., 60 characters) to avoid MAX_PATH issues on Windows.
    let mut truncated = sanitized;
    if truncated.len() > 60 {
        truncated.truncate(60);
    }
    // Trim trailing dots and spaces, which are invalid on Windows filesystems.
    truncated.trim_end_matches(|c| c == ' ' || c == '.').to_string()
}

/// Scan a downloads directory and populate a map of known downloads.
pub fn scan_downloads_dir(dir: &Path, downloads: &mut HashMap<String, DownloadStatus>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_downloads_dir(&path, downloads);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str == "mp3" || ext_str == "mp4" {
                            if let Some(video_id) = extract_video_id_from_path(&path) {
                                downloads.insert(video_id, DownloadStatus::Finished(path));
                            } else if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                downloads.insert(stem.to_string(), DownloadStatus::Finished(path));
                            }
                    }
                }
            }
        }
    }
}

use std::borrow::Cow;

/// Truncate a string to max_chars, adding an ellipsis if truncated. Returns a zero-copy reference when not truncated.
pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str> {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        truncated.push_str("...");
        Cow::Owned(truncated)
    } else {
        Cow::Borrowed(s)
    }
}

/// Format and print tabular data to stdout.
pub fn print_table<T>(
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

/// Construct the expected file path for a downloaded video/audio file.
pub fn get_download_path(
    downloads_base: &Path,
    channel_title: &str,
    video_title: &str,
    video_id: &str,
    is_audio: bool,
) -> PathBuf {
    let channel_dir_name = if channel_title.is_empty() {
        "Unknown Channel".to_string()
    } else {
        sanitize_filename(channel_title)
    };

    let ext = if is_audio { "mp3" } else { "mp4" };
    let file_name = format!("{} [{}].{}", sanitize_filename(video_title), video_id, ext);
    downloads_base.join(channel_dir_name).join(file_name)
}

/// Retrieve configured media player path from config if specified.
pub fn get_configured_player_path() -> Option<String> {
    let config = crate::load_config();
    config.player_path.filter(|s| !s.is_empty() && s != "ENTER_PATH_TO_MEDIA_PLAYER_HERE")
}

/// Launch an external media player for a file or URL target.
pub fn launch_external_player(target: &std::ffi::OsStr) -> Result<(), String> {
    let mut players = Vec::new();
    if let Some(user_player) = get_configured_player_path() {
        players.push(user_player);
    }
    players.extend(vec![
        "mpv".to_string(),
        "vlc".to_string(),
        "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe".to_string(),
        "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe".to_string(),
    ]);

    for player in players {
        if std::process::Command::new(&player).arg(target).spawn().is_ok() {
            return Ok(());
        }
    }

    Err("No media players succeeded.".to_string())
}

/// Load a set of strings from a JSON array file.
pub fn load_string_set_from_file(path: &Path) -> std::collections::HashSet<String> {
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(ids) = serde_json::from_str::<Vec<String>>(&content) {
                return ids.into_iter().collect();
            }
        }
    }
    std::collections::HashSet::new()
}

/// Save a set of strings to a JSON array file.
pub fn save_string_set_to_file(path: &Path, set: &std::collections::HashSet<String>) -> Result<(), String> {
    let list: Vec<&String> = set.iter().collect();
    let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}


