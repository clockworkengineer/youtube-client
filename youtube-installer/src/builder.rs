use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_release_binaries(install_cli: bool, install_gui: bool) -> (PathBuf, PathBuf) {
    println!("\nBuilding release binaries...");
    let cargo_available = Command::new("cargo").arg("--version").output().is_ok();
    if !cargo_available {
        println!("⚠️ Notice: 'cargo' was not detected in PATH. Checking for pre-built binaries...");
    } else {
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
                println!("⚠️ Cargo build failed or encountered an error. Looking for existing pre-built release binaries...");
            }
        }
    }

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

    (cli_src, gui_src)
}
