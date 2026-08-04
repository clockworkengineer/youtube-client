# YouTube Client Workspace

A modular, high-performance Rust workspace providing YouTube API integration, a native GUI application (`youtube-gui`), and a command-line interface (`youtube-client`).

---

## Workspace Structure

* **`youtube-client-lib`**: Core library handling YouTube Data API v3 authentication, OAuth tokens, subscriptions, feeds, playlists, comments, and media downloads.
* **`youtube-gui`**: Fast, lightweight native desktop application built with `egui` and `eframe`. Features persistent cleared video feeds, background audio playback via `rodio`, playlist management, and video streaming.
* **`youtube-client`**: Feature-rich CLI application for headless environments and command-line automation.

---

## Quick Start

### 1. Build the Workspace

```bash
cargo build --workspace --release
```

### 2. Run the Native GUI App

```bash
cargo run --bin youtube-gui
```

### 3. Run the CLI Client

```bash
cargo run --bin youtube-client -- subscriptions
```

---

## Features

* 🔐 **OAuth2 Desktop Authentication**: Secure browser-based authentication flow with token caching.
* 📺 **Subscriptions & Feeds**: Concurrent subscription fetching for fast feed load times.
* 🆕 **Persistent New Videos Feed**: Clear individual items or all videos with feed persistence across sessions.
* 🎵 **Built-in Audio Player**: Background audio playback worker utilizing `rodio`.
* 📥 **Media Downloading**: Download videos or audio streams directly.
* 📂 **Playlist & Comment Management**: View playlists, add items, and read video comments.

---

## Additional Guides

* [Google OAuth2 Credentials Setup Guide](docs/google_setup.md)
* [Video Playback Configuration Guide](docs/video_playback.md)
