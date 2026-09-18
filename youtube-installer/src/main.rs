use std::path::PathBuf;

mod builder;
mod config_writer;
mod platform;
mod prompt;

use builder::build_release_binaries;
use config_writer::setup_global_config;
use platform::{configure_platform_environment, remove_platform_environment};
use prompt::{get_default_install_dir, prompt};

const ICON_ICO_BYTES: &[u8] = include_bytes!("../../assets/icon.ico");
const ICON_PNG_BYTES: &[u8] = include_bytes!("../../assets/icon.png");

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let unattended = args.iter().any(|a| a == "-y" || a == "--yes");
    let target_dir_arg = args.windows(2).find(|w| w[0] == "--target-dir").map(|w| PathBuf::from(&w[1]));

    // Handle --uninstall mode
    if args.iter().any(|a| a == "--uninstall") {
        println!("====================================================");
        println!("         YouTube Client & GUI Uninstaller           ");
        println!("====================================================\n");

        let install_dir = if let Some(dir) = target_dir_arg {
            dir
        } else {
            get_default_install_dir()
        };

        println!("Target installation directory: {}", install_dir.display());

        let cli_bin_name = if cfg!(target_os = "windows") { "youtube-client.exe" } else { "youtube-client" };
        let gui_bin_name = if cfg!(target_os = "windows") { "youtube-gui.exe" } else { "youtube-gui" };

        let cli_file = install_dir.join(cli_bin_name);
        if cli_file.exists() {
            if std::fs::remove_file(&cli_file).is_ok() {
                println!("✓ Removed {}", cli_file.display());
            }
        }

        let gui_file = install_dir.join(gui_bin_name);
        if gui_file.exists() {
            if std::fs::remove_file(&gui_file).is_ok() {
                println!("✓ Removed {}", gui_file.display());
            }
        }

        let ico_file = install_dir.join("icon.ico");
        if ico_file.exists() {
            let _ = std::fs::remove_file(&ico_file);
        }
        let png_file = install_dir.join("icon.png");
        if png_file.exists() {
            let _ = std::fs::remove_file(&png_file);
        }

        remove_platform_environment(&install_dir);

        if install_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&install_dir) {
                if entries.count() == 0 {
                    let _ = std::fs::remove_dir(&install_dir);
                    println!("✓ Removed empty installation directory: {}", install_dir.display());
                }
            }
        }

        println!("\n====================================================");
        println!("🎉 Uninstallation completed successfully!");
        println!("====================================================");
        return Ok(());
    }

    // Handle --verify mode
    if args.iter().any(|a| a == "--verify") {
        println!("====================================================");
        println!("         YouTube Client & GUI Verification          ");
        println!("====================================================\n");

        let install_dir = if let Some(dir) = target_dir_arg {
            dir
        } else {
            get_default_install_dir()
        };

        println!("Checking installation directory: {}\n", install_dir.display());

        let cli_bin_name = if cfg!(target_os = "windows") { "youtube-client.exe" } else { "youtube-client" };
        let gui_bin_name = if cfg!(target_os = "windows") { "youtube-gui.exe" } else { "youtube-gui" };

        let cli_file = install_dir.join(cli_bin_name);
        if cli_file.exists() {
            println!("✓ CLI binary found: {}", cli_file.display());
            let status = std::process::Command::new(&cli_file)
                .arg("--help")
                .output();
            match status {
                Ok(out) if out.status.success() => println!("  ✓ CLI binary executes successfully (--help tested)"),
                Ok(out) => println!("  ⚠️ CLI binary returned non-zero code: {:?}", out.status),
                Err(e) => println!("  ❌ Failed to execute CLI binary: {}", e),
            }
        } else {
            println!("❌ CLI binary NOT found: {}", cli_file.display());
        }

        let gui_file = install_dir.join(gui_bin_name);
        if gui_file.exists() {
            println!("✓ GUI binary found: {}", gui_file.display());
        } else {
            println!("❌ GUI binary NOT found: {}", gui_file.display());
        }

        let ico_file = install_dir.join("icon.ico");
        if ico_file.exists() {
            println!("✓ Application icon found: {}", ico_file.display());
        }
        let png_file = install_dir.join("icon.png");
        if png_file.exists() {
            println!("✓ PNG icon found: {}", png_file.display());
        }

        if let Some(config_dir) = youtube_client_lib::get_global_config_dir() {
            let config_file = config_dir.join("config.json");
            if config_file.exists() {
                println!("✓ Global configuration found: {}", config_file.display());
            } else {
                println!("ℹ️ Global configuration not initialized at {}", config_file.display());
            }
        }

        println!("\n====================================================");
        println!("✓ Verification complete!");
        println!("====================================================");
        return Ok(());
    }

    println!("====================================================");
    println!("   Welcome to the YouTube Client & GUI Installer!   ");
    println!("====================================================\n");

    // 1. Get Installation Directory
    let install_dir = if let Some(dir) = target_dir_arg {
        println!("Using specified target directory: {}", dir.display());
        dir
    } else if unattended {
        let dir = get_default_install_dir();
        println!("Unattended mode: using default directory: {}", dir.display());
        dir
    } else {
        let default_dir = get_default_install_dir();
        let default_dir_str = default_dir.to_string_lossy();
        let install_dir_input = prompt("Enter installation directory", &default_dir_str);
        PathBuf::from(install_dir_input)
    };

    // 2. Select Components
    let (install_cli, install_gui) = if unattended {
        println!("Unattended mode: installing both CLI and GUI components.");
        (true, true)
    } else {
        println!("\nSelect components to install:");
        println!("  1. Both CLI Client and GUI Application (Recommended)");
        println!("  2. CLI Client Only");
        println!("  3. GUI Application Only");
        let component_choice = prompt("Select option (1-3)", "1");
        let cli = component_choice == "1" || component_choice == "2";
        let gui = component_choice == "1" || component_choice == "3";
        (cli, gui)
    };

    // 3. Build Binaries
    let (cli_src, gui_src) = build_release_binaries(install_cli, install_gui);

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

        // Install icons
        let ico_dest = install_dir.join("icon.ico");
        let png_dest = install_dir.join("icon.png");
        if std::fs::write(&ico_dest, ICON_ICO_BYTES).is_ok() {
            println!("✓ Installed application icon: {}", ico_dest.display());
        }
        if std::fs::write(&png_dest, ICON_PNG_BYTES).is_ok() {
            println!("✓ Installed PNG icon: {}", png_dest.display());
        }
    }

    // 5. Initialize Configuration
    setup_global_config()?;

    // 6. Platform specific integrations (PATH & Shortcuts)
    configure_platform_environment(&install_dir, install_gui);

    println!("\n====================================================");
    println!("🎉 Installation completed successfully!");
    println!("====================================================");
    Ok(())
}
