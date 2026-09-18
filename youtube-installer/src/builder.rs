use std::path::{Path, PathBuf};
use std::process::Command;

fn find_binary(name: &str) -> Option<PathBuf> {
    let bin_name = if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    // 1. Check directory of currently running installer executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let candidate = dir.join(&bin_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 2. Check current working directory and relative target/release
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join(&bin_name);
        if candidate.exists() {
            return Some(candidate);
        }
        let candidate_target = cwd.join("target").join("release").join(&bin_name);
        if candidate_target.exists() {
            return Some(candidate_target);
        }
    }

    // 3. Check compile-time workspace target/release (for developer cargo run)
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        if let Some(parent) = Path::new(manifest_dir).parent() {
            let candidate = parent.join("target").join("release").join(&bin_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn build_release_binaries(install_cli: bool, install_gui: bool) -> (PathBuf, PathBuf) {
    let cli_name = "youtube-client";
    let gui_name = "youtube-gui";

    let cli_found = if install_cli {
        find_binary(cli_name)
    } else {
        None
    };
    let gui_found = if install_gui {
        find_binary(gui_name)
    } else {
        None
    };

    let need_cli_build = install_cli && cli_found.is_none();
    let need_gui_build = install_gui && gui_found.is_none();

    if !need_cli_build && !need_gui_build {
        println!("✓ Using pre-built release binaries found in package distribution.");
        let cli_path = cli_found.unwrap_or_else(|| PathBuf::from(cli_name));
        let gui_path = gui_found.unwrap_or_else(|| PathBuf::from(gui_name));
        return (cli_path, gui_path);
    }

    println!("\nBuilding release binaries from source via cargo...");
    let cargo_available = Command::new("cargo").arg("--version").output().is_ok();
    if !cargo_available {
        println!(
            "⚠️ Notice: 'cargo' was not detected in PATH, and pre-built binaries were not found."
        );
    } else {
        let mut cargo_cmd = Command::new("cargo");
        cargo_cmd.arg("build").arg("--release");
        if need_cli_build {
            cargo_cmd.arg("--package").arg("youtube-client");
        }
        if need_gui_build {
            cargo_cmd.arg("--package").arg("youtube-gui");
        }

        let status = cargo_cmd.status();
        match status {
            Ok(s) if s.success() => {
                println!("✓ Successfully built release binaries!");
            }
            _ => {
                println!("⚠️ Cargo build encountered an error. Looking for fallback binaries...");
            }
        }
    }

    let cli_final = find_binary(cli_name).unwrap_or_else(|| {
        PathBuf::from(if cfg!(target_os = "windows") {
            "youtube-client.exe"
        } else {
            "youtube-client"
        })
    });
    let gui_final = find_binary(gui_name).unwrap_or_else(|| {
        PathBuf::from(if cfg!(target_os = "windows") {
            "youtube-gui.exe"
        } else {
            "youtube-gui"
        })
    });

    (cli_final, gui_final)
}
