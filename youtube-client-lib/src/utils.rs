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

/// Truncate a string to max_chars, adding an ellipsis if truncated.
pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        s.to_string()
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

