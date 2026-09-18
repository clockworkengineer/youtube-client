use std::io::{self, Write};
use std::path::PathBuf;

pub fn prompt(message: &str, default: &str) -> String {
    print!("{message} [{default}]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        default.to_string()
    }
}

pub fn get_default_install_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            PathBuf::from(local_appdata)
                .join("Programs")
                .join("youtube-client")
        } else {
            PathBuf::from("C:\\Program Files\\youtube-client")
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local").join("bin")
    } else {
        PathBuf::from("/usr/local/bin")
    }
}
