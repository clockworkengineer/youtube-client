# `youtube-client` (v0.2.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![CLI: clap](https://img.shields.io/badge/CLI-clap%20v4-blue.svg)](https://github.com/clap-rs/clap)
[![Subcommands: 23](https://img.shields.io/badge/subcommands-23-brightgreen.svg)](../docs/CLI_REFERENCE.md)

A feature-rich command-line interface for querying YouTube Data API v3, managing subscriptions and playlists, searching videos with filters, exporting/importing OPML/CSV/JSON backups, auditing offline media libraries and watch history, rating media, and downloading audio/video streams.

---

## Features

* **23 Subcommands:** Exhaustive coverage of YouTube API v3 and local library/history operations directly from the terminal.
* **Advanced Search & Download Filters:** Filter searches by sort order (`--order`), duration (`--duration`), and content type (`--type`); constrain downloads by video resolution (`--resolution 1080p`) and audio bitrate (`--audio-quality 320`).
* **Subscription Backup & Migration:** Export and import subscriptions across OPML 2.0 XML, Google Takeout CSV, and NewPipe JSON (`export-subscriptions`, `import-subscriptions`).
* **Offline Media Library & Watch History:** Inspect local downloaded audio/video files (`library --audio-only`) and manage watch progress history (`history --limit 10`, `--remove`, `--clear`).
* **Pluggable Output Formatters:** Decoupled `OutputFormatter<T>` engine supporting aligned terminal tables, pretty JSON (`--json`), and CSV export.
* **Zero-OAuth API Key & Headless Device-Code Login:** Query public endpoints with `--api-key` or authenticate over SSH with `login --device-code`.
* **Offline Mock Injection:** Run commands against in-memory mock fixtures using `CliContext::with_mock` for fast integration testing.
* **Return YouTube Dislike (RYD):** Restored dislike metrics in the `details` subcommand output.
* **Shell Auto-Completions:** Generate tab-completion scripts for `bash`, `zsh`, `fish`, `powershell`, and `elvish`.

---

## Quick Reference Cheatsheet

```bash
# 1. Authenticate with Google OAuth2 (browser or headless device-code)
youtube-client login
youtube-client login --device-code

# 2. Search for videos with sort and duration filters
youtube-client search --query "Rust async" --order view-count --duration medium --limit 5

# 3. Stream all subscriptions as JSON for piping to jq
youtube-client subscriptions --all --json | jq '.[].title'

# 4. Export and import subscriptions (OPML, CSV, NewPipe JSON)
youtube-client export-subscriptions --output subscriptions.opml
youtube-client import-subscriptions --file subscriptions.csv

# 5. View video details with RYD dislike metrics
youtube-client details --video-id dQw4w9WgXcQ

# 6. Download 320 kbps MP3 audio or 1080p MP4 video
youtube-client download --video-id dQw4w9WgXcQ --format mp3 --audio-quality 320
youtube-client download --video-id dQw4w9WgXcQ --format mp4 --resolution 1080p

# 7. Browse offline media library and watch history
youtube-client library --audio-only
youtube-client history --limit 10

# 8. Generate shell completion script for Bash
youtube-client completions bash > ~/.local/share/bash-completion/completions/youtube-client
```

---

## Documentation

* [CLI Reference Manual (23 Subcommands)](../docs/CLI_REFERENCE.md)
* [Subscription Backup & Migration Guide](../docs/BACKUP_AND_MIGRATION.md)
* [Configuration & Secrets Guide](../docs/CONFIGURATION.md)
* [SOLID Architecture](../docs/solid_ARCHITECTURE.md)
