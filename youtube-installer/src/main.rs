use std::path::PathBuf;

mod builder;
mod config_writer;
mod platform;
mod prompt;

use builder::build_release_binaries;
use config_writer::setup_global_config;
use platform::configure_platform_environment;
use prompt::{get_default_install_dir, prompt};

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
