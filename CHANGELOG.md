# Changelog

All notable changes to the `youtube-client` workspace are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - Complete Feature Set Implementation & Comprehensive Documentation Suite

### Added
- **Extended YouTube API V3 Models & Core Library (`youtube-client-lib`)**:
  - `VideoDetails`: Comprehensive model with engagement metrics (view count, like count, comment count), ISO 8601 duration parser (`parse_iso8601_duration`), formatted duration (`format_duration`), and topic tags.
  - `ChannelDetails`: Channel profile model with subscriber count, total video count, and cumulative view counts.
  - `fetch_video_details(&self, video_id: &str)` on `YoutubeClient` and `VideoService`.
  - `get_channel_details(&self, channel_id: &str)` on `YoutubeClient`.
  - `post_comment(&self, video_id: &str, text: &str)` on `YoutubeClient` and `CommentService`.
  - `delete_playlist(&self, playlist_id: &str)` on `YoutubeClient` and `PlaylistService`.
  - Full mock support in `MockYoutubeClient` for all new services and endpoints.
- **CLI Feature Expansion (`youtube-client`)**:
  - `details <VIDEO_ID>`: Display comprehensive video statistics and engagement metrics with `--json` option.
  - `channel <CHANNEL_ID>`: Display channel subscriber, video, and view count metrics with `--json` option.
  - `comments <VIDEO_ID>`: List top-level video comments with author details and like counts.
  - `comment-post <VIDEO_ID> <TEXT>`: Interactive and headless posting of comments to YouTube videos.
  - `subscribe <CHANNEL_ID>` and `unsubscribe <SUBSCRIPTION_ID>`: Direct channel subscription management.
  - `playlist-create <TITLE> [--description <DESC>]`: Direct creation of custom YouTube playlists.
  - `playlist-delete <PLAYLIST_ID>`: Removal of custom playlists with confirmation prompt and `-y` flag.
  - 8 new automated integration tests covering all added CLI subcommands.
- **Native GUI Enhancements (`youtube-gui`)**:
  - `VideoDetails` view enriched with statistics badges (views, likes, comments, duration), clickable channel link navigating to channel feed, and interactive comment submission form.
  - `Playlists` view upgraded with "➕ Create New Playlist" inline form and per-playlist "🗑 Delete" button.
  - `Subscriptions` view upgraded with real-time channel title & ID filter search box.
  - Audio playback panel upgraded with volume slider (`0%` to `100%`) and quick mute/unmute toggle.
- **Installer & Environment Lifecycle (`youtube-installer`)**:
  - `--verify` flag to validate binary integrity, executable status, and global configuration setup.
  - `--uninstall` flag to cleanly remove binaries, Start Menu shortcuts, desktop entries, and user PATH modifications.

## [0.1.1] - Quality Attributes Architecture Refactor

This milestone refactored the entire workspace to implement the **10 Attributes of a Well-Written Software Library & System**:

### Added
- **API Ergonomics & Types (`youtube-client-lib`)**:
  - Strongly typed `Rating` enum (`Like`, `Dislike`, `None`) with `FromStr` and `Display` implementations.
  - Generic `Page<T>` pagination model with `.has_more()`, `.len()`, `.is_empty()`, and helper constructors.
  - `YoutubeClientBuilder` fluent builder pattern for configuring credentials, scopes, and custom OAuth delegates.
  - `DownloadFormat` enum (`Mp4`, `Mp3`, `BestAudio`, `Custom(String)`) and `DownloadOptions` struct for fine-grained media downloads.
  - Granular `YoutubeError` enum with specialized error variants (`Auth`, `Api`, `Network`, `RateLimit`, `Download`, `Io`, `Json`, `InvalidInput`).
  - Unit test mocking harness `MockYoutubeClient` implementing `VideoService`, `PlaylistService`, `SubscriptionService`, `CommentService`, and `MediaDownloader`.
- **Reliability & Resilience**:
  - `retry_api_call` with exponential backoff, randomized jitter, and quota-exceeded detection.
  - Atomic JSON file writing via temporary file replacement (`write_json_atomically`) to prevent corruption from abrupt terminations or power loss.
  - Hardened filename sanitization protecting against Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`), CLI injection flags (leading dashes), Unicode length limits, and null bytes.
- **Cross-Platform Support**:
  - Standardized configuration directories across macOS (`~/Library/Application Support`), Linux (`$XDG_CONFIG_HOME`), and Windows (`APPDATA`).
  - Cross-platform media player path detection (`#[cfg(target_os)]`) supporting Windows (`mpv.exe`, `vlc.exe`), macOS (`/Applications/VLC.app`), and Linux (`/usr/bin/mpv`, `/usr/bin/vlc`).
- **Performance & UI Responsiveness (`youtube-gui`)**:
  - Texture LRU cache with eviction tracking (`VecDeque`) to prevent unbounded VRAM growth.
  - Eliminated unconditional 60 FPS state vector clones in `eframe::update()`, replacing with on-demand view state extraction and poisoned lock recovery.
- **CLI Client Overhaul (`youtube-client`)**:
  - Expanded command suite with `search`, `rate`, and `playlists` subcommands.
  - Added `--json` flag to `subscriptions`, `videos`, `search`, and `playlists` for shell piping and scripting.
  - Added `--all` exhaustive pagination and `--page-token` resumption tokens.
  - Added `DownloadFormat` options (`--format mp3|mp4|bestaudio`), quality constraints, and custom yt-dlp arguments.
  - Encapsulated command arguments into `CliContext` with `YoutubeClientBuilder` integration.
  - Added automated CLI integration test suite (`tests/cli_tests.rs`) with `clap::Command::debug_assert`.
- **GUI & Installer Improvements**:
  - Moved media downloads scanning in `youtube-gui` to a background task, ensuring sub-16ms startup latency.
  - Hardened `youtube-installer` with pre-flight `cargo` detection and unattended mode (`--yes` / `-y` and `--target-dir`).
- **Modular Library Architecture**:
  - Decomposed the 994-line monolithic `youtube-client-lib/src/lib.rs` into focused modules: `client`, `builder`, `download`, `audio`, `retry`, `error`, `models`, `traits`, `utils`, `config`, and `testing`.
  - Converted `youtube-client-lib` default features to `[]` (opt-in), reducing compile times and unnecessary dependencies for headless consumers.
- **Removed**:
  - Dead dependency `rusty_ytdl = "0.7.4"` removed from workspace root and crates, eliminating redundant compilation overhead.
