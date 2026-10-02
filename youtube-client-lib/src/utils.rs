//! # Shared Utilities & Helper Primitives
//!
//! Provides filename sanitization, video ID extraction from paths, string set persistence,
//! table printing, external player invocation, and media download status tracking.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Represents the download state of a video.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadStatus {
    NotStarted,
    Downloading { progress: String },
    Finished(PathBuf),
    Failed(String),
}

/// Extract a YouTube video ID from a file path (expects a stem of length 11 or bracketed `[ID]`).
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

/// Windows reserved device names that cannot be used as filenames or stems.
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitize a filename for safe, cross-platform filesystem usage.
///
/// Features:
/// - Strips illegal characters: `['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0']`
/// - Strips leading dashes to prevent command-line option injection in external utilities
/// - Guards against Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`)
/// - Truncates Unicode characters safely to a maximum of 60 characters
/// - Trims invalid trailing dots and spaces
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => c,
        })
        .collect();

    // Trim trailing dots and spaces, which are invalid on Windows filesystems
    let trimmed = sanitized.trim_end_matches([' ', '.']);

    // Strip leading dashes and spaces to prevent argument injection in subprocesses
    let no_leading = trimmed.trim_start_matches(['-', ' ']);

    let effective_name = if no_leading.is_empty() {
        "unnamed"
    } else {
        no_leading
    };

    // Check Windows reserved names
    let stem = effective_name.split('.').next().unwrap_or(effective_name);
    let is_reserved = WINDOWS_RESERVED_NAMES
        .iter()
        .any(|&res| res.eq_ignore_ascii_case(stem));

    let mut result = if is_reserved {
        format!("{effective_name}_")
    } else {
        effective_name.to_string()
    };

    // Safely truncate to max 60 Unicode chars to avoid MAX_PATH restrictions
    if result.chars().count() > 60 {
        result = result.chars().take(60).collect();
        result = result.trim_end_matches([' ', '.']).to_string();
    }

    if result.is_empty() {
        "unnamed".to_string()
    } else {
        result
    }
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

/// Format tabular data into an aligned, column-padded String.
pub fn format_table<T>(
    headers: &[&str],
    widths: &[usize],
    items: &[T],
    row_formatter: impl Fn(&T, usize) -> Vec<String>,
) -> String {
    if items.is_empty() {
        return "No items found.".to_string();
    }

    let mut out = String::new();
    for (i, header) in headers.iter().enumerate() {
        out.push_str(&format!("{:<width$} ", header, width = widths[i]));
    }
    out.push('\n');

    let total_width: usize = widths.iter().sum::<usize>() + widths.len() - 1;
    out.push_str(&"-".repeat(total_width));
    out.push('\n');

    for (idx, item) in items.iter().enumerate() {
        let cols = row_formatter(item, idx);
        for (i, col) in cols.iter().enumerate() {
            out.push_str(&format!("{:<width$} ", col, width = widths[i]));
        }
        out.push('\n');
    }
    out.trim_end_matches('\n').to_string()
}

/// Format and print tabular data to stdout.
pub fn print_table<T>(
    headers: &[&str],
    widths: &[usize],
    items: &[T],
    row_formatter: impl Fn(&T, usize) -> Vec<String>,
) {
    println!("{}", format_table(headers, widths, items, row_formatter));
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
    config
        .player_path
        .filter(|s| !s.is_empty() && s != "ENTER_PATH_TO_MEDIA_PLAYER_HERE")
}

/// Launch an external media player for a file or URL target with cross-platform fallbacks.
pub fn launch_external_player(target: &std::ffi::OsStr) -> Result<(), String> {
    launch_external_player_with_log(target, None)
}

/// Launch an external media player for a file or URL target with cross-platform fallbacks and logging.
pub fn launch_external_player_with_log(
    target: &std::ffi::OsStr,
    log_file: Option<&Path>,
) -> Result<(), String> {
    launch_external_player_with_options(target, log_file, None)
}

/// Launch an external media player for a file or URL target with start offset position and logging.
pub fn launch_external_player_with_options(
    target: &std::ffi::OsStr,
    log_file: Option<&Path>,
    start_secs: Option<f32>,
) -> Result<(), String> {
    let opts = crate::player::PlayOptions {
        start_secs,
        cookies_file: crate::config::resolve_cookies_file(None),
        cookies_from_browser: crate::config::resolve_cookies_from_browser(None),
        log_file: log_file.map(|p| p.to_path_buf()),
        title: None,
    };
    let registry = crate::player::PlayerRegistry::with_defaults();
    registry.launch(target, &opts)
}

/// Playback position and duration metadata for resume-from-last-watched features.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct PlaybackProgress {
    /// Playback position in seconds
    pub position_secs: f32,
    /// Total duration in seconds (if known)
    pub duration_secs: f32,
    /// Unix timestamp in seconds when last updated
    pub updated_at: u64,
}

/// Load playback progress map (`video_id -> PlaybackProgress`) from a JSON file.
pub fn load_playback_positions_from_file(
    path: &Path,
) -> std::collections::HashMap<String, PlaybackProgress> {
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<
                std::collections::HashMap<String, PlaybackProgress>,
            >(&content)
            {
                return map;
            }
        }
    }
    std::collections::HashMap::new()
}

/// Save playback progress map to a JSON file atomically using a temporary file.
pub fn save_playback_positions_to_file(
    path: &Path,
    positions: &std::collections::HashMap<String, PlaybackProgress>,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(positions).map_err(|e| e.to_string())?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() && !parent.exists() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;

    Ok(())
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

/// Save a set of strings to a JSON array file atomically using a temporary file.
pub fn save_string_set_to_file(
    path: &Path,
    set: &std::collections::HashSet<String>,
) -> Result<(), String> {
    let list: Vec<&String> = set.iter().collect();
    let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() && !parent.exists() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;

    Ok(())
}

fn format_current_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let sec = (total_secs % 60) as u32;
    let total_mins = total_secs / 60;
    let min = (total_mins % 60) as u32;
    let total_hours = total_mins / 60;
    let hour = (total_hours % 24) as u32;
    let mut days = (total_hours / 24) as i64;

    // Howard Hinnant's algorithm for Gregorian date calculation from epoch days
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hour:02}:{min:02}:{sec:02}")
}

/// Append a line with a timestamp and prefix to a log file, creating any missing parent directories.
pub fn append_to_log(path: &Path, prefix: &str, message: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let now = format_current_timestamp();
        let _ = writeln!(file, "[{now}] [{prefix}] {message}");
    }
}

/// Query the public Return YouTube Dislike (RYD) API for estimated video dislike counts.
pub async fn fetch_dislike_count(video_id: &str) -> Option<u64> {
    let url = format!("https://returnyoutubedislikeapi.com/votes?videoId={video_id}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if resp.status().is_success() {
        #[derive(serde::Deserialize)]
        struct RydResponse {
            dislikes: Option<u64>,
        }
        if let Ok(ryd) = resp.json::<RydResponse>().await {
            return ryd.dislikes;
        }
    }
    None
}
