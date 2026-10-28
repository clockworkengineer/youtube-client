# Installation, Packaging & Lifecycle Guide (v0.2.0)

This guide explains how to build, install, verify, package, and uninstall the YouTube Client suite (`youtube-client`, `youtube-gui`, and `youtube-installer`) using the native `youtube-installer` utility, the Inno Setup Windows installer, or GitHub Actions release artifacts.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Distribution & Installation Options](#distribution--installation-options)
   - [Option A: Inno Setup Windows Installer (`setup.exe`)](#option-a-inno-setup-windows-installer-setupexe)
   - [Option B: Native `youtube-installer` Utility](#option-b-native-youtube-installer-utility)
   - [Option C: GitHub Actions Pre-Compiled Releases](#option-c-github-actions-pre-compiled-releases)
3. [System Integration Details](#system-integration-details)
4. [Verifying an Installation (`--verify`)](#verifying-an-installation---verify)
5. [Clean Uninstallation (`--uninstall`)](#clean-uninstallation---uninstall)
6. [Manual Compilation & Optimization](#manual-compilation--optimization)

---

## Prerequisites

* **Rust Toolchain:** Stable Rust (1.80+) and Cargo (for building from source).
* **C Compiler / Build Essentials:**
  * **Windows:** MSVC C++ build tools (Visual Studio or Build Tools for Visual Studio).
  * **Linux:** `build-essential`, `pkg-config`, `libasound2-dev` (for Rodio audio).
  * **macOS:** Xcode Command Line Tools (`xcode-select --install`).
* **Runtime Tools:**
  * `yt-dlp`: Required for media downloading and external streaming.
  * `mpv` or `vlc`: For native external video playback.

---

## Distribution & Installation Options

### Option A: Inno Setup Windows Installer (`setup.exe`)

For Windows users, the repository provides an Inno Setup script ([`dist/windows/setup.iss`](file:///c:/Projects/youtube-client/dist/windows/setup.iss)) that compiles a single-file, professional installer: `youtube-client-setup-0.2.0.exe`.

#### Features:
* Automatic architecture detection (x86_64).
* Destination directory selection (defaults to `C:\Program Files\YouTube Client`).
* Desktop and Start Menu shortcut tasks.
* Automatic `PATH` registration.
* Standard Windows Add/Remove Programs registration with uninstaller.

#### Compiling the Installer:
1. Build release binaries:
   ```powershell
   cargo build --workspace --release
   ```
2. Run Inno Setup Compiler:
   ```powershell
   iscc dist\windows\setup.iss
   ```
   The compiled setup executable is generated in `dist\windows\Output\youtube-client-setup-0.2.0.exe`.

---

### Option B: Native `youtube-installer` Utility

The repository includes a dedicated cross-platform installer crate (`youtube-installer`):

#### 1. Interactive Installation
```bash
cargo run --bin youtube-installer
```
* Prompts for installation directory (defaults: `%LOCALAPPDATA%\Programs\YouTubeClient` on Windows, `~/.local/bin` on Linux/macOS).
* Compiles release binaries automatically.
* Adds destination to user `PATH` (`~/.bashrc`, `~/.zshrc`, or Windows Registry).
* Deploys desktop shortcuts and `.desktop` application entries.
* Configures global `config.json`.

#### 2. Unattended / Automated Installation (`--yes`)
For headless systems, containers, or scripted setup:
```bash
cargo run --bin youtube-installer -- --yes --target-dir "C:\Tools\YouTubeClient"
```

---

### Option C: GitHub Actions Pre-Compiled Releases

The project includes an automated release workflow ([`.github/workflows/release.yml`](file:///c:/Projects/youtube-client/.github/workflows/release.yml)) triggered on version tags (`v*`):
* Compiles release binaries across `windows-latest`, `ubuntu-latest`, and `macos-latest`.
* Packages portable `.zip` archives containing `youtube-client`, `youtube-gui`, `youtube-installer`, and documentation.
* Compiles `youtube-client-setup-<version>.exe` on Windows.
* Generates SHA256 cryptographic checksums for all release artifacts.
* Publishes assets directly to the GitHub Release page.

---

## System Integration Details

### Windows Integration
* **Executable Location:** Copied to target directory alongside icons.
* **Shortcuts:** Created in Start Menu (`YouTube Client`) and Desktop.
* **Registry:** Registered in `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\YouTubeClient`.
* **User PATH:** Appended to `HKCU\Environment\PATH`.

### Linux & macOS Integration
* **Application Icons:** Installs `icon.png` into target directory.
* **User PATH:** Appends `export PATH="<INSTALL_DIR>:$PATH"` to `~/.bashrc`, `~/.zshrc`, or `~/.profile`.
* **Desktop Entry:** Writes `~/.local/share/applications/youtube-gui.desktop` linked to the custom icon for desktop menu integration (GNOME, KDE, XFCE).

---

## Verifying an Installation (`--verify`)

To verify that your installation is intact and binaries execute properly:

```bash
youtube-installer --verify
```

### Verification Checks Performed:
* ✓ CLI binary presence in target installation directory.
* ✓ CLI `--help` invocation test (verifies dynamic linking and clap setup).
* ✓ GUI binary presence in target installation directory.
* ✓ Installer / Uninstaller utility presence in target installation directory.
* ✓ Custom application icon files (`icon.ico`, `icon.png`).
* ✓ Global configuration directory and credentials file accessibility.

---

## Clean Uninstallation (`--uninstall`)

The installer provides complete uninstallation capability:

```bash
youtube-installer --uninstall
```

### Actions Performed:
1. Deletes `youtube-client`, `youtube-gui`, and `youtube-installer` executables.
2. Deletes `icon.ico` and `icon.png`.
3. Removes Windows Add/Remove Programs registry entry.
4. Removes Desktop and Start Menu shortcuts.
5. Cleans the target directory from user `PATH`.
6. Removes empty parent directories.

---

## Manual Compilation & Optimization

The root `Cargo.toml` specifies aggressive size and performance optimizations for release builds:

```toml
[profile.release]
opt-level = "s"     # Optimize for size while maintaining speed
lto = true          # Link-Time Optimization across crates
codegen-units = 1   # Single code generation unit for maximum dead-code elimination
panic = "abort"     # Remove unwinding tables for smaller binary footprint
strip = true        # Strip debug symbols automatically
```

To compile release binaries manually:
```bash
cargo build --workspace --release
```
Binaries are placed in `target/release/`:
* `target/release/youtube-client` (CLI)
* `target/release/youtube-gui` (GUI)
* `target/release/youtube-installer` (Installer)
