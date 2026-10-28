# `youtube-gui` (v0.3.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%200.26-blue.svg)](https://github.com/emilk/egui)

A fast, lightweight, and modern 60 FPS desktop graphical user interface for YouTube built with `egui` and `eframe`, engineered around **Jakob Nielsen's 10 Usability Heuristics**.

---

## Features

* **7 Primary Navigation Views (`Alt+1..7`):**
  1. **`📺 Subscriptions` (`Alt+1`):** Real-time channel filter, rich channel headers, and confirmed unsubscribe safeguards.
  2. **`🆕 New Videos` (`Alt+2`):** Aggregated uploads across subscriptions with `↩ Undo Dismiss (N)` and zero-quota RSS fallback.
  3. **`📂 Playlists` (`Alt+3`):** Create, edit, and delete playlists, or remove individual playlist items with confirmation.
  4. **`📁 Library` (`Alt+4`):** Offline downloaded media manager with `All` / `🎵 Audio` / `🎬 Video` filter tabs, storage summary, `▶ Play`, `🎧 Listen`, `📂 Show in Folder`, and confirmed deletion.
  5. **`⏱ History` (`Alt+5`):** Chronological watch progress history (`playback_positions.json`) with relative timestamps (`Watched 5m ago`), completion bars, `▶ Resume`, and `🔄 Restart`.
  6. **`⚙ Settings` (`Alt+6`):** Configure preferred player, default video resolution & audio bitrate, SponsorBlock categories, browser cookies, API quota gauge, and OPML/CSV/JSON subscription backup.
  7. **`ℹ About & Help` (`Alt+7`):** Built-in keyboard shortcuts reference and external dependency (`yt-dlp`, `ffmpeg`, `mpv`, `vlc`) health check.
* **One-Click `🎧 Listen` Audio Streaming & Dock:** Extracts and caches `rodio`-compatible `.mp3` / `.m4a` streams into `%TEMP%/youtube-client-audio/` with live buffering percentages (`🎧 Buffering audio... (N%)`), `⏹ Cancel Audio` (`CancellationToken`), timeline scrubber slider, `⏪ 10s` / `⏩ 10s` skip buttons, and volume/mute controls.
* **SponsorBlock Timeline Visualization:** Proportional color-coded segment bar in Video Details (`sponsor`, `selfpromo`, `interaction`, `intro`, `outro`) with `MM:SS – MM:SS` hover tooltips and native MPV skipping.
* **Selectable Video Resolution & Audio Bitrate Menus:** Choose target video resolution (`best`, `2160p`..`360p`) and MP3 bitrate (`best`, `320k`..`128k`) per download in Video Details.
* **Automated Error Diagnosis & Recovery Banners:** Translates raw OAuth, quota, network, and `yt-dlp` errors into plain-English guidance with one-click recovery buttons (`🔑 Re-authenticate`, `⚙ Open Settings`, `🔄 Retry`, `📋 Copy Error`).
* **Return YouTube Dislike (RYD):** Restored community dislike metrics and like/dislike ratio bars.

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

| Shortcut | Action |
| :--- | :--- |
| `Alt + 1` .. `Alt + 7` | Switch directly between the 7 primary views (`Subscriptions`, `New Videos`, `Playlists`, `Library`, `History`, `Settings`, `About & Help`) |
| `Ctrl + F` or `/` | Focus the top search bar from any screen |
| `F5` or `Ctrl + R` | Refresh the currently active view |
| `Esc` or `Alt + Left` | Navigate back in the view history stack or dismiss an open confirmation prompt |
| `Space` | Toggle Play/Pause on the active background audio stream (when no text field is focused) |
| `Ctrl + S` | Save settings immediately when in the `⚙ Settings` view |

---

## Documentation

* [Desktop GUI User Guide](../docs/GUI_USER_GUIDE.md)
* [Usability Heuristics & Keyboard Shortcuts Guide](../docs/USABILITY_AND_SHORTCUTS.md)
* [Video & Audio Playback Configuration](../docs/VIDEO_PLAYBACK.md)
* [Subscription Backup & Migration Guide](../docs/BACKUP_AND_MIGRATION.md)
* [Configuration & Secrets Guide](../docs/CONFIGURATION.md)
