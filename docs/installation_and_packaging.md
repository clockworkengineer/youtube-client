# Installation, Packaging & Lifecycle Guide (`youtube-installer`)

This guide explains how to build, install, verify, and uninstall the YouTube Client suite (`youtube-client` and `youtube-gui`) using the native `youtube-installer` utility or manual Cargo commands.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Interactive Installation](#interactive-installation)
3. [Unattended / Automated Installation](#unattended--automated-installation)
4. [System Integration Details](#system-integration-details)
5. [Verifying an Installation (`--verify`)](#verifying-an-installation---verify)
6. [Clean Uninstallation (`--uninstall`)](#clean-uninstallation---uninstall)
7. [Manual Building & Packaging](#manual-building--packaging)

---

## Prerequisites

* **Rust Toolchain:** Stable Rust (1.80+) and Cargo.
* **C Compiler / Build Essentials:**
  * **Windows:** MSVC C++ build tools (Visual Studio or Build Tools for Visual Studio).
  * **Linux:** `build-essential`, `pkg-config`, `libasound2-dev` (for Rodio audio).
  * **macOS:** Xcode Command Line Tools (`xcode-select --install`).
* **Optional Runtime Tools:**
  * `yt-dlp`: Required for media downloading and MPV YouTube streaming.
  * `mpv` or `vlc`: For native video playback.

---

## Interactive Installation

The repository includes a dedicated installer crate (`youtube-installer`) that automates release compilation, binary copying, configuration setup, and desktop integration:

```bash
cargo run --bin youtube-installer
```

### Installation Steps
1. The installer prompts for an installation destination directory.
   * **Default Windows:** `%LOCALAPPDATA%\Programs\YouTubeClient`
   * **Default Linux / macOS:** `~/.local/bin` or `~/bin`
2. Prompts to build release binaries (`youtube-client` and `youtube-gui`).
3. Prompts to add the destination folder to your user `PATH`.
4. Prompts to create Start Menu and Desktop shortcuts (Windows) or `.desktop` application entries (Linux).
5. Prompts to set up global `config.json` with your Google Client credentials.

---

## Unattended / Automated Installation

For scripted setups, headless systems, or automated deployment scripts, `youtube-installer` supports unattended mode flags:

```bash
cargo run --bin youtube-installer -- --yes --target-dir "C:\Tools\YouTubeClient"
```

### Unattended Flags

| Flag | Description |
| :--- | :--- |
| `-y, --yes` | Accept all default prompts automatically without interactive prompts. |
| `--target-dir <DIR>` | Specify target installation directory directly. |

---

## System Integration Details

### Windows Integration
* **Application Icons:** Installs high-resolution multi-format `icon.ico` and `icon.png` to the installation folder and embeds the custom icon in `youtube-gui.exe`.
* **User PATH Modification:** Adds the installation directory to `HKCU\Environment\Path` in the Windows Registry and broadcasts `WM_SETTINGCHANGE` so new terminals immediately recognize `youtube-client` without requiring a reboot.
* **Start Menu Shortcut:** Creates `YouTube Client GUI.lnk` in `%APPDATA%\Microsoft\Windows\Start Menu\Programs` configured with the custom icon.
* **Desktop Shortcut:** Creates `YouTube Client GUI.lnk` on the user's Desktop with the custom application icon for instant access.

### Linux / macOS Integration
* **Application Icons:** Installs `icon.png` into the installation directory.
* **User PATH:** Appends `export PATH="<INSTALL_DIR>:$PATH"` to `~/.bashrc`, `~/.zshrc`, or `~/.profile`.
* **Desktop Entry:** Writes `~/.local/share/applications/youtube-gui.desktop` linked to the custom icon for seamless integration with desktop application menus (GNOME, KDE, XFCE).

---

## Verifying an Installation (`--verify`)

To verify that your installation is intact, binaries are executable, and configuration is accessible, run:

```bash
youtube-installer --verify
```
Or via cargo:
```bash
cargo run --bin youtube-installer -- --verify
```

### Verification Checks Performed:
* ✓ CLI binary presence in target installation directory.
* ✓ CLI `--help` invocation test (verifies execution and dynamic linker).
* ✓ GUI binary presence in target installation directory.
* ✓ Global configuration directory and credentials file accessibility.

---

## Clean Uninstallation (`--uninstall`)

The installer provides complete uninstallation capability, removing all deployed artifacts:

```bash
youtube-installer --uninstall
```
Or with custom target directory:
```bash
youtube-installer --uninstall --target-dir "C:\Tools\YouTubeClient"
```

### Actions Performed During Uninstallation:
1. Deletes `youtube-client` and `youtube-gui` executables.
2. Removes Desktop and Start Menu shortcuts.
3. Cleans the target directory from user `PATH` (Windows Registry or Unix shell rc files).
4. Deletes the installation directory if it is empty.

---

## Manual Building & Packaging

If you prefer building standalone binaries without the installer:

```bash
cargo build --workspace --release
```

### Compiler Optimization Profiles

The root `Cargo.toml` specifies aggressive size and performance optimizations for release builds:

```toml
[profile.release]
opt-level = "s"     # Optimize for size while maintaining speed
lto = true          # Link-Time Optimization across crates
codegen-units = 1   # Single code generation unit for maximum dead-code elimination
panic = "abort"     # Remove unwinding tables for smaller binary footprint
strip = true        # Strip debug symbols automatically
```

Binaries will be output to `target/release/`:
* `target/release/youtube-client` (CLI)
* `target/release/youtube-gui` (GUI)
* `target/release/youtube-installer` (Installer)
