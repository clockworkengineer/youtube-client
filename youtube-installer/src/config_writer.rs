pub fn setup_global_config() -> anyhow::Result<()> {
    if let Some(config_dir) = youtube_client_lib::get_global_config_dir() {
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
