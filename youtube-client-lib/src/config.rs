//! # Configuration & Credentials Management
//!
//! Handles loading client secrets, environment variables, config JSON files,
//! and verifying Google OAuth2 scope caches.

use std::path::Path;

/// Application configuration settings loaded from `private_config.json` or `config.json`.
#[derive(serde::Deserialize, Clone, Debug, Default)]
pub struct Config {
    /// Google OAuth2 Client ID
    pub client_id: Option<String>,
    /// Google OAuth2 Client Secret
    pub client_secret: Option<String>,
    /// Path to preferred external media player
    pub player_path: Option<String>,
    /// Path to downloads directory
    pub downloads_dir: Option<String>,
}

impl Config {
    /// Check whether client credentials are non-empty and non-placeholder.
    pub fn is_valid(&self) -> bool {
        let invalid_id = |id: &str| id.is_empty() || id == "ENTER_YOUR_CLIENT_ID_HERE";
        let invalid_secret = |sec: &str| sec.is_empty() || sec == "ENTER_YOUR_CLIENT_SECRET_HERE";

        self.client_id.as_deref().map_or(false, |id| !invalid_id(id))
            && self.client_secret.as_deref().map_or(false, |sec| !invalid_secret(sec))
    }
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
        std::env::var_os("APPDATA").map(|appdata| std::path::PathBuf::from(appdata).join("youtube-client"))
    } else {
        std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config").join("youtube-client"))
    }
}

pub fn load_config() -> Config {
    let local = load_config_from_dir(Path::new("."));
    let mut merged = local.clone();
    
    if !local.is_valid() {
        if let Some(global_dir) = get_global_config_dir() {
            let global = load_config_from_dir(&global_dir);
            if merged.client_id.is_none() {
                merged.client_id = global.client_id;
            }
            if merged.client_secret.is_none() {
                merged.client_secret = global.client_secret;
            }
            if merged.player_path.is_none() {
                merged.player_path = global.player_path;
            }
            if merged.downloads_dir.is_none() {
                merged.downloads_dir = global.downloads_dir;
            }
        }
    } else if let Some(global_dir) = get_global_config_dir() {
        let global = load_config_from_dir(&global_dir);
        if merged.player_path.is_none() {
            merged.player_path = global.player_path;
        }
        if merged.downloads_dir.is_none() {
            merged.downloads_dir = global.downloads_dir;
        }
    }
    merged
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

    let temp_config = Config {
        client_id: cid,
        client_secret: csec,
        player_path: None,
        downloads_dir: None,
    };

    if temp_config.is_valid() {
        Ok((temp_config.client_id.unwrap(), temp_config.client_secret.unwrap()))
    } else {
        Err(crate::YoutubeError::Credentials(format!(
            "Google Client ID and Client Secret must be provided!\n\n{}",
            GOOGLE_SETUP_INSTRUCTIONS
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
                            if required_scopes.iter().any(|&s| s == scope_str) {
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
