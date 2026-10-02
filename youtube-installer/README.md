# `youtube-installer` (v0.2.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)

A standalone system installation, health verification, and uninstallation utility for the YouTube Client suite (`youtube-client` and `youtube-gui`).

---

## Features

* **Interactive Setup:** Guided prompts for installation directory, release compilation, user `PATH` configuration, desktop shortcuts, and global configuration.
* **Unattended Automated Mode (`--yes`):** Fully non-interactive deployments for headless systems, Docker containers, and automated setup scripts.
* **Health Verification (`--verify`):** Comprehensive integrity checks verifying binary executability, clap command line arguments, icons, and configuration accessibility.
* **Clean Uninstallation (`--uninstall`):** Deletes binaries, icons, system shortcuts, and unregisters from user `PATH` and Windows Add/Remove Programs.

---

## Usage

### Interactive Installation
```bash
cargo run --bin youtube-installer
```

### Unattended / Scripted Installation
```bash
cargo run --bin youtube-installer -- --yes --target-dir "C:\Tools\YouTubeClient"
```

### Health Verification
```bash
youtube-installer --verify
```

### Uninstallation
```bash
youtube-installer --uninstall
```

---

## Documentation

* [Installation, Packaging & Lifecycle Guide](../docs/installation_and_packaging.md)
* [Configuration Guide](../docs/configuration.md)
