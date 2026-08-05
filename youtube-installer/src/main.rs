use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

fn prompt(message: &str, default: &str) -> String {
    print!("{} [{}]: ", message, default);
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

fn get_default_install_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            PathBuf::from(local_appdata).join("Programs").join("youtube-client")
        } else {
            PathBuf::from("C:\\Program Files\\youtube-client")
        }
    } else {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".local").join("bin")
        } else {
            PathBuf::from("/usr/local/bin")
        }
    }
}

fn main() -> anyhow::Result<()> {
    println!("====================================================");
    println!("   Welcome to the YouTube Client & GUI Installer!   ");
    println!("====================================================\n");

    // 1. Get Installation Directory
    let default_dir = get_default_install_dir();
    let default_dir_str = default_dir.to_string_lossy();
    let install_dir_input = prompt("Enter installation directory", &default_dir_str);
    let install_dir = PathBuf::from(install_dir_input);

    // 2. Select Components
    println!("\nSelect components to install:");
    println!("  1. Both CLI Client and GUI Application (Recommended)");
    println!("  2. CLI Client Only");
    println!("  3. GUI Application Only");
    let component_choice = prompt("Select option (1-3)", "1");

    let install_cli = component_choice == "1" || component_choice == "2";
    let install_gui = component_choice == "1" || component_choice == "3";

    // 3. Build Binaries
    println!("\nBuilding release binaries...");
    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.arg("build").arg("--release");
    if install_cli && !install_gui {
        cargo_cmd.arg("--package").arg("youtube-client");
    } else if install_gui && !install_cli {
        cargo_cmd.arg("--package").arg("youtube-gui");
    } else {
        cargo_cmd.arg("--workspace");
    }

    let status = cargo_cmd.status();
    match status {
        Ok(s) if s.success() => {
            println!("✓ Successfully built release binaries!");
        }
        _ => {
            println!("⚠️ Cargo build failed or not found. Looking for existing pre-built release binaries...");
        }
    }

    // Determine binary files
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let target_dir = workspace_root.join("target").join("release");

    let cli_src = if cfg!(target_os = "windows") {
        target_dir.join("youtube-client.exe")
    } else {
        target_dir.join("youtube-client")
    };

    let gui_src = if cfg!(target_os = "windows") {
        target_dir.join("youtube-gui.exe")
    } else {
        target_dir.join("youtube-gui")
    };

    // Ensure installation folder exists
    std::fs::create_dir_all(&install_dir)?;

    // 4. Copy Binaries
    if install_cli {
        if cli_src.exists() {
            let cli_dest = install_dir.join(cli_src.file_name().unwrap());
            std::fs::copy(&cli_src, &cli_dest)?;
            println!("✓ Copied youtube-client to: {}", cli_dest.display());
        } else {
            println!("❌ Error: youtube-client binary not found at {}", cli_src.display());
        }
    }

    if install_gui {
        if gui_src.exists() {
            let gui_dest = install_dir.join(gui_src.file_name().unwrap());
            std::fs::copy(&gui_src, &gui_dest)?;
            println!("✓ Copied youtube-gui to: {}", gui_dest.display());
        } else {
            println!("❌ Error: youtube-gui binary not found at {}", gui_src.display());
        }
    }

    // 5. Initialize Configuration
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
            println!("✓ Created global config template at: {}", config_file.display());
        } else {
            println!("✓ Global config file already exists at: {}", config_file.display());
        }
    }

    // 6. Platform specific integrations (PATH & Shortcuts)
    if cfg!(target_os = "windows") {
        // Update user PATH
        println!("\nSetting up environment PATH variable...");
        let install_path_str = install_dir.to_string_lossy().to_string();
        let add_path_script = format!(
            "$oldPath = [Environment]::GetEnvironmentVariable('Path', 'User'); \
             if ($oldPath -notlike '*{}*') {{ \
                 [Environment]::SetEnvironmentVariable('Path', $oldPath + ';{}', 'User'); \
                 Write-Host '✓ Added to user PATH' \
             }} else {{ \
                 Write-Host '✓ PATH already configured' \
             }}",
            install_path_str.replace("\\", "\\\\"),
            install_path_str.replace("\\", "\\\\")
        );
        let _ = Command::new("powershell")
            .arg("-Command")
            .arg(&add_path_script)
            .status();

        // Create Start Menu Shortcut
        if install_gui {
            if let Some(home) = std::env::var_os("USERPROFILE") {
                let start_menu = PathBuf::from(home)
                    .join("AppData")
                    .join("Roaming")
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs");
                if start_menu.exists() {
                    let shortcut_path = start_menu.join("YouTube Client GUI.lnk");
                    let gui_exe_path = install_dir.join("youtube-gui.exe");
                    println!("Creating Start Menu shortcut...");
                    let shortcut_script = format!(
                        "$WshShell = New-Object -ComObject WScript.Shell; \
                         $Shortcut = $WshShell.CreateShortcut('{}'); \
                         $Shortcut.TargetPath = '{}'; \
                         $Shortcut.WorkingDirectory = '{}'; \
                         $Shortcut.Save()",
                        shortcut_path.to_string_lossy().replace("\\", "\\\\"),
                        gui_exe_path.to_string_lossy().replace("\\", "\\\\"),
                        install_dir.to_string_lossy().replace("\\", "\\\\")
                    );
                    let shortcut_status = Command::new("powershell")
                        .arg("-Command")
                        .arg(&shortcut_script)
                        .status();
                    if shortcut_status.map_or(false, |s| s.success()) {
                        println!("✓ Created Start Menu shortcut!");
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        // Create Desktop file
        if install_gui {
            if let Some(home) = std::env::var_os("HOME") {
                let apps_dir = PathBuf::from(home).join(".local").join("share").join("applications");
                if apps_dir.exists() {
                    let desktop_file_path = apps_dir.join("youtube-gui.desktop");
                    let gui_path = install_dir.join("youtube-gui");
                    let desktop_content = format!(
                        "[Desktop Entry]\n\
                         Type=Application\n\
                         Name=YouTube Client GUI\n\
                         Comment=Native YouTube Desktop client\n\
                         Exec={}\n\
                         Icon=video-television\n\
                         Terminal=false\n\
                         Categories=Utility;AudioVideo;\n",
                        gui_path.display()
                    );
                    if std::fs::write(&desktop_file_path, desktop_content).is_ok() {
                        println!("✓ Created desktop entry in {}", desktop_file_path.display());
                    }
                }
            }
        }

        // Print PATH warning
        let path_var = std::env::var("PATH").unwrap_or_default();
        let install_path_str = install_dir.to_string_lossy();
        if !path_var.contains(&*install_path_str) {
            println!("\n⚠️  Please make sure '{}' is added to your PATH.", install_path_str);
            println!("   You can do this by adding the following line to your ~/.bashrc or ~/.zshrc:");
            println!("   export PATH=\"$PATH:{}\"", install_path_str);
        }
    } else if cfg!(target_os = "macos") {
        // macOS instructions
        let path_var = std::env::var("PATH").unwrap_or_default();
        let install_path_str = install_dir.to_string_lossy();
        if !path_var.contains(&*install_path_str) {
            println!("\n⚠️  Please make sure '{}' is added to your PATH.", install_path_str);
            println!("   export PATH=\"$PATH:{}\"", install_path_str);
        }
    }

    println!("\n====================================================");
    println!("🎉 Installation completed successfully!");
    println!("====================================================");
    Ok(())
}
