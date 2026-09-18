use std::path::{Path, PathBuf};
use std::process::Command;

pub fn configure_platform_environment(install_dir: &Path, install_gui: bool) {
    if cfg!(target_os = "windows") {
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

        if install_gui {
            let icon_ico_path = install_dir.join("icon.ico");
            let icon_ico_str = icon_ico_path.to_string_lossy().replace("\\", "\\\\");
            let gui_exe_path = install_dir.join("youtube-gui.exe");
            let gui_exe_str = gui_exe_path.to_string_lossy().replace("\\", "\\\\");
            let install_dir_str = install_dir.to_string_lossy().replace("\\", "\\\\");

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
                    println!("Creating Start Menu shortcut with custom icon...");
                    let shortcut_script = format!(
                        "$WshShell = New-Object -ComObject WScript.Shell; \
                         $Shortcut = $WshShell.CreateShortcut('{}'); \
                         $Shortcut.TargetPath = '{}'; \
                         $Shortcut.WorkingDirectory = '{}'; \
                         $Shortcut.IconLocation = '{},0'; \
                         $Shortcut.Save()",
                        shortcut_path.to_string_lossy().replace("\\", "\\\\"),
                        gui_exe_str,
                        install_dir_str,
                        icon_ico_str
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

            // Also create Desktop shortcut on Windows
            let desktop_script = format!(
                "$WshShell = New-Object -ComObject WScript.Shell; \
                 $desktop = [Environment]::GetFolderPath('Desktop'); \
                 if ($desktop) {{ \
                     $Shortcut = $WshShell.CreateShortcut(\"$desktop\\YouTube Client GUI.lnk\"); \
                     $Shortcut.TargetPath = '{}'; \
                     $Shortcut.WorkingDirectory = '{}'; \
                     $Shortcut.IconLocation = '{},0'; \
                     $Shortcut.Save(); \
                     Write-Host '✓ Created Desktop shortcut!' \
                 }}",
                gui_exe_str,
                install_dir_str,
                icon_ico_str
            );
            let _ = Command::new("powershell")
                .arg("-Command")
                .arg(&desktop_script)
                .status();
        }
    } else if cfg!(target_os = "linux") {
        if install_gui {
            if let Some(home) = std::env::var_os("HOME") {
                let apps_dir = PathBuf::from(home).join(".local").join("share").join("applications");
                if apps_dir.exists() {
                    let desktop_file_path = apps_dir.join("youtube-gui.desktop");
                    let gui_path = install_dir.join("youtube-gui");
                    let icon_png_path = install_dir.join("icon.png");
                    let desktop_content = format!(
                        "[Desktop Entry]\n\
                         Type=Application\n\
                         Name=YouTube Client GUI\n\
                         Comment=Native YouTube Desktop client\n\
                         Exec={}\n\
                         Icon={}\n\
                         Terminal=false\n\
                         Categories=Utility;AudioVideo;\n",
                        gui_path.display(),
                        icon_png_path.display()
                    );
                    if std::fs::write(&desktop_file_path, desktop_content).is_ok() {
                        println!("✓ Created desktop entry with custom icon in {}", desktop_file_path.display());
                    }
                }
            }
        }
    } else if cfg!(target_os = "macos") {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let install_path_str = install_dir.to_string_lossy();
        if !path_var.contains(&*install_path_str) {
            println!("\n⚠️  Please make sure '{}' is added to your PATH.", install_path_str);
            println!("   You can do this by adding the following line to your ~/.bashrc or ~/.zshrc:");
            println!("   export PATH=\"$PATH:{}\"", install_path_str);
        }
    }
}

pub fn remove_platform_environment(install_dir: &Path) {
    if cfg!(target_os = "windows") {
        println!("Removing environment PATH entry...");
        let install_path_str = install_dir.to_string_lossy().to_string();
        let remove_path_script = format!(
            "$path = [Environment]::GetEnvironmentVariable('Path', 'User'); \
             if ($path) {{ \
                 $parts = $path.Split(';') | Where-Object {{ $_ -ne '{}' -and $_ -ne '' }}; \
                 [Environment]::SetEnvironmentVariable('Path', ($parts -join ';'), 'User'); \
                 Write-Host '✓ Removed from user PATH' \
             }}",
            install_path_str.replace("\\", "\\\\")
        );
        let _ = Command::new("powershell")
            .arg("-Command")
            .arg(&remove_path_script)
            .status();

        if let Some(home) = std::env::var_os("USERPROFILE") {
            let shortcut_path = PathBuf::from(home)
                .join("AppData")
                .join("Roaming")
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("YouTube Client GUI.lnk");
            if shortcut_path.exists() {
                if std::fs::remove_file(&shortcut_path).is_ok() {
                    println!("✓ Removed Start Menu shortcut.");
                }
            }
        }

        let remove_desktop_script = "\
            $desktop = [Environment]::GetFolderPath('Desktop'); \
            if ($desktop) { \
                $file = \"$desktop\\YouTube Client GUI.lnk\"; \
                if (Test-Path $file) { Remove-Item -Force $file; Write-Host '✓ Removed Desktop shortcut.' } \
            }";
        let _ = Command::new("powershell")
            .arg("-Command")
            .arg(remove_desktop_script)
            .status();
    } else if cfg!(target_os = "linux") {
        if let Some(home) = std::env::var_os("HOME") {
            let desktop_file_path = PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("applications")
                .join("youtube-gui.desktop");
            if desktop_file_path.exists() {
                if std::fs::remove_file(&desktop_file_path).is_ok() {
                    println!("✓ Removed desktop entry.");
                }
            }
        }
    }
}

