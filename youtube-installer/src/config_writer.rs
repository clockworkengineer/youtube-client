use std::path::PathBuf;

pub fn get_global_config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("youtube-client"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("youtube-client")
        })
    } else {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(PathBuf::from(xdg).join("youtube-client"))
        } else {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config").join("youtube-client"))
        }
    }
}

pub fn setup_global_config() -> anyhow::Result<()> {
    if let Some(config_dir) = get_global_config_dir() {
        std::fs::create_dir_all(&config_dir)?;
        let config_file = config_dir.join("config.json");
        if !config_file.exists() {
            let default_config = serde_json::json!({
                "client_id": "ENTER_YOUR_CLIENT_ID_HERE",
                "client_secret": "ENTER_YOUR_CLIENT_SECRET_HERE",
                "player_path": null,
                "downloads_dir": null
            });
            std::fs::write(&config_file, serde_json::to_string_pretty(&default_config)?)?;
            println!(
                "✓ Created global config template at: {}",
                config_file.display()
            );
        } else {
            println!(
                "✓ Global config file already exists at: {}",
                config_file.display()
            );
        }
    }
    Ok(())
}
