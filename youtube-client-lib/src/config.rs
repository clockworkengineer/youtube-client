//! # Configuration & Credentials Management
//!
//! Handles loading client secrets, environment variables, config JSON files,
//! and verifying Google OAuth2 scope caches.

use std::path::Path;

/// Application configuration settings loaded from `private_config.json` or `config.json`.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct Config {
    /// Google OAuth2 Client ID
    pub client_id: Option<String>,
    /// Google OAuth2 Client Secret
    pub client_secret: Option<String>,
    /// Path to preferred external media player
    pub player_path: Option<String>,
    /// Path to downloads directory
    pub downloads_dir: Option<String>,
    /// Path to client log file
    pub log_file: Option<String>,
    /// Path to cookies.txt file for yt-dlp/MPV authentication
    pub cookies_file: Option<String>,
    /// Browser name to extract cookies from (e.g. "chrome", "firefox", "edge", "brave")
    pub cookies_from_browser: Option<String>,
    /// Persistent audio volume level (0.0 to 1.0)
    pub volume: Option<f32>,
    /// Persistent window position [x, y] in screen coordinates
    pub window_pos: Option<[f32; 2]>,
    /// Persistent window inner size [width, height] in logical points
    pub window_size: Option<[f32; 2]>,
    /// Persistent window maximized state
    pub window_maximized: Option<bool>,
}

/// Resolve the path for the client log file.
/// Precedence:
/// 1. Explicit CLI parameter override (if provided)
/// 2. `YOUTUBE_CLIENT_LOG_FILE` environment variable
/// 3. `log_file` entry in configuration file
/// 4. Default: `"youtube-client.log"`
pub fn resolve_log_file_path(cli_override: Option<&Path>) -> std::path::PathBuf {
    if let Some(path) = cli_override {
        return path.to_path_buf();
    }
    if let Ok(env_path) = std::env::var("YOUTUBE_CLIENT_LOG_FILE") {
        if !env_path.trim().is_empty() {
            return std::path::PathBuf::from(env_path.trim());
        }
    }
    let config = load_config();
    if let Some(ref cfg_path) = config.log_file {
        if !cfg_path.trim().is_empty() {
            return std::path::PathBuf::from(cfg_path.trim());
        }
    }
    resolve_app_data_path("youtube-client.log")
}

/// Resolve path to cookies file if specified via CLI, environment variable, or configuration.
pub fn resolve_cookies_file(cli_override: Option<&Path>) -> Option<std::path::PathBuf> {
    if let Some(path) = cli_override {
        return Some(path.to_path_buf());
    }
    if let Ok(env_path) = std::env::var("YOUTUBE_COOKIES_FILE") {
        let trimmed = env_path.trim();
        if !trimmed.is_empty() {
            return Some(std::path::PathBuf::from(trimmed));
        }
    }
    let config = load_config();
    if let Some(ref path) = config.cookies_file {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(std::path::PathBuf::from(trimmed));
        }
    }
    None
}

/// Resolve browser name for cookie extraction if specified via CLI, environment variable, or configuration.
pub fn resolve_cookies_from_browser(cli_override: Option<&str>) -> Option<String> {
    if let Some(b) = cli_override {
        let trimmed = b.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(env_browser) = std::env::var("YOUTUBE_COOKIES_FROM_BROWSER") {
        let trimmed = env_browser.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let config = load_config();
    if let Some(ref b) = config.cookies_from_browser {
        let trimmed = b.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

impl Config {
    /// Check whether client credentials are non-empty and non-placeholder.
    pub fn is_valid(&self) -> bool {
        let invalid_id = |id: &str| id.is_empty() || id == "ENTER_YOUR_CLIENT_ID_HERE";
        let invalid_secret = |sec: &str| sec.is_empty() || sec == "ENTER_YOUR_CLIENT_SECRET_HERE";

        self.client_id.as_deref().is_some_and(|id| !invalid_id(id))
            && self
                .client_secret
                .as_deref()
                .is_some_and(|sec| !invalid_secret(sec))
    }

    /// Check whether client credentials are valid either directly or via built-in default credentials.
    pub fn is_resolvable(&self) -> bool {
        self.is_valid() || has_default_credentials()
    }
}

/// Default embedded Google OAuth client credentials for YouTube desktop client.
/// Can be configured at build time via DEFAULT_GOOGLE_CLIENT_ID and DEFAULT_GOOGLE_CLIENT_SECRET environment variables.
pub const DEFAULT_CLIENT_ID: Option<&str> = match option_env!("DEFAULT_GOOGLE_CLIENT_ID") {
    Some(val) => Some(val),
    None => Some("474926444117-b6osuhgvik71cgqp2o928atth9d80mgj.apps.googleusercontent.com"),
};

pub const DEFAULT_CLIENT_SECRET: Option<&str> = match option_env!("DEFAULT_GOOGLE_CLIENT_SECRET") {
    Some(val) => Some(val),
    None => match std::str::from_utf8(&[
        71, 79, 67, 83, 80, 88, 45, 113, 79, 86, 121, 85, 120, 107, 89, 115, 115, 86, 54, 75, 73,
        74, 88, 121, 81, 87, 53, 70, 82, 98, 102, 90, 103, 50, 80,
    ]) {
        Ok(s) => Some(s),
        Err(_) => None,
    },
};

/// Retrieve default embedded OAuth credentials if available and non-empty.
pub fn get_default_credentials() -> Option<(&'static str, &'static str)> {
    match (DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET) {
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => Some((id, secret)),
        _ => None,
    }
}

/// Check whether default credentials are configured.
pub fn has_default_credentials() -> bool {
    get_default_credentials().is_some()
}

pub const GOOGLE_SETUP_INSTRUCTIONS: &str = "\
Please configure them in one of the following ways:\n\
1. Pass them as arguments: --client-id <ID> --client-secret <SECRET>\n\
2. Set the GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables\n\
3. Create a config file (config.json) with client credentials, e.g.:\n\
   {\n\
     \"client_id\": \"your_id_here\",\n\
     \"client_secret\": \"your_secret_here\"\n\
   }\n\n\
To get Google API Client credentials:\n\
1. Go to the Google Cloud Console: https://console.cloud.google.com/\n\
2. Create a project and search for the \"YouTube Data API v3\" and enable it.\n\
3. Navigate to \"APIs & Services\" > \"Credentials\".\n\
4. Click \"Create Credentials\" > \"OAuth client ID\". Choose \"Desktop app\".\n\
5. Retrieve your Client ID and Client Secret.";

pub fn get_global_config_dir() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(|appdata| std::path::PathBuf::from(appdata).join("youtube-client"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| {
            std::path::PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("youtube-client")
        })
    } else {
        // Linux / BSD: Check XDG_CONFIG_HOME first, fallback to ~/.config
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(std::path::PathBuf::from(xdg).join("youtube-client"))
        } else {
            std::env::var_os("HOME").map(|home| {
                std::path::PathBuf::from(home)
                    .join(".config")
                    .join("youtube-client")
            })
        }
    }
}

/// Resolve the path for an application state, cache, or data file.
/// Precedence:
/// 1. Local path if it already exists in the current working directory (preserves portability in development).
/// 2. Global application directory if the file exists there.
/// 3. Global application directory if available (creating parent directory if needed).
/// 4. Local path as final fallback.
pub fn resolve_app_data_path(file_name: &str) -> std::path::PathBuf {
    let local = std::path::PathBuf::from(file_name);
    if local.exists() {
        return local;
    }
    if let Some(global_dir) = get_global_config_dir() {
        let global_file = global_dir.join(file_name);
        if global_file.exists() {
            return global_file;
        }
        let _ = std::fs::create_dir_all(&global_dir);
        return global_file;
    }
    local
}

/// Secure a sensitive file (like `tokencache.json`) by setting Unix permissions to 0600 (owner read/write only).
pub fn secure_sensitive_file(_path: &Path) {
    #[cfg(unix)]
    if _path.exists() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o600));
    }
}

/// Resolve the path to `tokencache.json`.
/// Checks current working directory first; if not present there, falls back to the global application directory.
pub fn resolve_token_cache_path() -> std::path::PathBuf {
    let path = resolve_app_data_path("tokencache.json");
    secure_sensitive_file(&path);
    path
}

pub fn load_config() -> Config {
    let local = load_config_from_dir(Path::new("."));
    let mut merged = local.clone();

    if let Some(global_dir) = get_global_config_dir() {
        let global = load_config_from_dir(&global_dir);
        if !local.is_valid() {
            if merged.client_id.is_none() {
                merged.client_id = global.client_id;
            }
            if merged.client_secret.is_none() {
                merged.client_secret = global.client_secret;
            }
        }
        if merged.player_path.is_none() {
            merged.player_path = global.player_path;
        }
        if merged.downloads_dir.is_none() {
            merged.downloads_dir = global.downloads_dir;
        }
        if merged.log_file.is_none() {
            merged.log_file = global.log_file;
        }
        if merged.cookies_file.is_none() {
            merged.cookies_file = global.cookies_file;
        }
        if merged.cookies_from_browser.is_none() {
            merged.cookies_from_browser = global.cookies_from_browser;
        }
        if merged.volume.is_none() {
            merged.volume = global.volume;
        }
        if merged.window_pos.is_none() {
            merged.window_pos = global.window_pos;
        }
        if merged.window_size.is_none() {
            merged.window_size = global.window_size;
        }
        if merged.window_maximized.is_none() {
            merged.window_maximized = global.window_maximized;
        }
    }
    merged
}

/// Save configuration to the appropriate `config.json` file.
/// If `config.json` already exists in the current working directory, it is updated there.
/// Otherwise, it is saved into the global user config directory (`%APPDATA%/youtube-client/config.json` or `~/.config/youtube-client/config.json`).
pub fn save_config(config: &Config) -> std::io::Result<()> {
    let local_file = Path::new("config.json");
    let target_file = if local_file.exists() {
        local_file.to_path_buf()
    } else if let Some(global_dir) = get_global_config_dir() {
        std::fs::create_dir_all(&global_dir)?;
        global_dir.join("config.json")
    } else {
        local_file.to_path_buf()
    };

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&target_file, json)
}

pub fn load_config_from_dir(dir: &Path) -> Config {
    let private_config = dir.join("private_config.json");
    let fallback_config = dir.join("config.json");

    let config_path = if private_config.exists() {
        private_config
    } else {
        fallback_config
    };

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<Config>(&content) {
                return config;
            }
        }
    }
    Config::default()
}

pub fn load_config_from_file_or_default(path: &Path) -> Config {
    if path != Path::new("config.json") {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<Config>(&content) {
                return config;
            }
        }
    }
    load_config()
}

/// Resolve YouTube API credentials from explicit options, environment variables, or configuration files.
pub fn resolve_credentials(
    opt_client_id: Option<String>,
    opt_client_secret: Option<String>,
    config_path: &Path,
) -> std::result::Result<(String, String), crate::YoutubeError> {
    let mut cid = opt_client_id.or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok());
    let mut csec = opt_client_secret.or_else(|| std::env::var("GOOGLE_CLIENT_SECRET").ok());

    if cid.is_none() || csec.is_none() {
        let config = load_config_from_file_or_default(config_path);
        if cid.is_none() {
            cid = config.client_id;
        }
        if csec.is_none() {
            csec = config.client_secret;
        }
    }

    let is_empty_or_placeholder = |val: Option<&String>| {
        val.is_none_or(|s| {
            s.is_empty() || s == "ENTER_YOUR_CLIENT_ID_HERE" || s == "ENTER_YOUR_CLIENT_SECRET_HERE"
        })
    };

    if (is_empty_or_placeholder(cid.as_ref()) || is_empty_or_placeholder(csec.as_ref()))
        && has_default_credentials()
    {
        if let Some((default_id, default_sec)) = get_default_credentials() {
            cid = Some(default_id.to_string());
            csec = Some(default_sec.to_string());
        }
    }

    let temp_config = Config {
        client_id: cid,
        client_secret: csec,
        ..Default::default()
    };

    if temp_config.is_valid() {
        Ok((
            temp_config.client_id.unwrap(),
            temp_config.client_secret.unwrap(),
        ))
    } else {
        Err(crate::YoutubeError::Credentials(format!(
            "Google Client ID and Client Secret must be provided!\n\n{GOOGLE_SETUP_INSTRUCTIONS}"
        )))
    }
}

pub fn check_token_cache_scopes(token_cache_path: &Path, required_scopes: &[&str]) -> bool {
    if let Ok(content) = std::fs::read_to_string(token_cache_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            let check_scopes = |scopes: &serde_json::Value| -> bool {
                if let Some(arr) = scopes.as_array() {
                    for scope in arr {
                        if let Some(scope_str) = scope.as_str() {
                            if required_scopes.contains(&scope_str) {
                                return true;
                            }
                        }
                    }
                }
                false
            };

            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(scopes) = item.get("scopes") {
                        if check_scopes(scopes) {
                            return true;
                        }
                    }
                    if let Some(opts) = item.get("opts") {
                        if let Some(scopes) = opts.get("scopes") {
                            if check_scopes(scopes) {
                                return true;
                            }
                        }
                    }
                }
            } else if let Some(obj) = val.as_object() {
                for (_key, cache_val) in obj {
                    if let Some(scopes) = cache_val.get("scopes") {
                        if check_scopes(scopes) {
                            return true;
                        }
                    }
                    if let Some(opts) = cache_val.get("opts") {
                        if let Some(scopes) = opts.get("scopes") {
                            if check_scopes(scopes) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
