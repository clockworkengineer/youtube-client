# `youtube-client` (v0.2.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![CLI: clap](https://img.shields.io/badge/CLI-clap%20v4-blue.svg)](https://github.com/clap-rs/clap)

A feature-rich command-line interface for querying YouTube Data API v3, managing subscriptions and playlists, searching videos, rating media, and downloading audio/video streams.

---

## Features

* **17 Subcommands:** Exhaustive coverage of YouTube operations directly from the terminal.
* **Pluggable Output Formatters:** Decoupled `OutputFormatter<T>` engine supporting aligned terminal tables, pretty JSON (`--json`), and CSV export.
* **Offline Mock Injection:** Run commands against in-memory mock fixtures using `CliContext::with_mock` for fast integration testing.
* **Return YouTube Dislike (RYD):** Restored dislike metrics in the `details` subcommand output.
* **Shell Auto-Completions:** Generate tab-completion scripts for `bash`, `zsh`, `fish`, `powershell`, and `elvish`.

---

## Quick Reference Cheatsheet

```bash
# 1. Authenticate with Google OAuth2
youtube-client login

# 2. Search for videos
youtube-client search --query "Rust async" --limit 5

# 3. Stream all subscriptions as JSON for piping to jq
youtube-client subscriptions --all --json | jq '.[].title'

# 4. View video details with RYD dislike metrics
youtube-client details --video-id dQw4w9WgXcQ

# 5. List comments on a video
youtube-client comments --video-id dQw4w9WgXcQ --limit 10

# 6. Download audio as MP3
youtube-client download --video-id dQw4w9WgXcQ --format mp3

# 7. Generate shell completion script for Bash
youtube-client completions bash > ~/.local/share/bash-completion/completions/youtube-client
```

---

## Documentation

* [CLI Reference Manual](../docs/cli_reference.md)
* [Configuration & Secrets Guide](../docs/configuration.md)
* [SOLID Architecture](../docs/solid_architecture.md)
