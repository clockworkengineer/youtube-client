# `youtube-gui` (v0.2.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%200.26-blue.svg)](https://github.com/emilk/egui)

A fast, lightweight, and modern desktop graphical user interface for YouTube built with `egui` and `eframe`.

---

## Features

* **Clean Desktop UI:** Smooth immediate-mode 60 FPS rendering powered by `eframe` and `egui`.
* **Decoupled Domain Handlers:** Refactored under SOLID principles with dedicated action handlers (`handlers/auth`, `handlers/playback`, `handlers/settings`, `handlers/library`, etc.).
* **Persistent Watch Progress:** Automatically saves watch positions per video ID to `playback_positions.json`, rendering watched badges (`⏱ Watched to MM:SS`) and instant `▶ Resume` controls.
* **Integrated Audio Dock:** Background audio playback via `rodio` with timeline scrubber slider, 10s skip buttons (`⏪ 10s`, `⏩ 10s`), duration readout (`MM:SS / MM:SS`), and volume controls.
* **Native In-App Settings:** Manage preferred media player (Auto, MPV, VLC, Custom), download folder picker, browser cookie extraction, and one-click account disconnection.
* **Return YouTube Dislike (RYD):** Restored community dislike metrics and tooltips across feed cards and video details.
* **Subscription Import & Export:** Built-in OPML export and multi-format (OPML, CSV, JSON) auto-detecting subscription importer.
* **Feed Clearance:** Dismiss single videos or clear entire feeds with atomic state persistence (`cleared_videos.json`).
* **Persistent Window Geometry:** Automatically remembers window position, size, and maximized state across sessions.

---

## Running the Application

### Via Cargo
```bash
cargo run --bin youtube-gui
```

### With Custom Log File
```bash
cargo run --bin youtube-gui -- --log-file custom_client.log
```

---

## Keyboard Navigation & Shortcuts

* `Ctrl + Q` / `Cmd + Q`: Exit application.
* `Space`: Toggle Play/Pause on the active audio stream.
* `F5`: Refresh current view.

---

## Documentation

* [Desktop GUI User Guide](../docs/gui_user_guide.md)
* [Video & Audio Playback Configuration](../docs/video_playback.md)
* [Configuration & Secrets Guide](../docs/configuration.md)
