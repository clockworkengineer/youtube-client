# Changelog

All notable changes to the `youtube-client` workspace are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - Release Readiness, Robust Storage Architecture & In-App Settings

### Added
- **Application State & Data Path Architecture (`youtube-client-lib`)**:
  - Centralized `resolve_app_data_path(file_name)` ensuring `cleared_videos.json`, `tokencache.json`, and `youtube-client.log` are stored in standard OS application data directories (`%APPDATA%/youtube-client` on Windows, `~/.config/youtube-client` on Linux/macOS) with automatic directory creation, eliminating data loss and permission errors when launched from shortcuts or non-terminal environments.
  - Implemented `secure_sensitive_file` enforcing `0600` permissions on Unix platforms for token caches.
  - Added `serde::Serialize` derive on `Config` struct to support dynamic configuration saving.
- **Native Settings View & Account Management (`youtube-gui`)**:
  - Added dedicated `Settings` view allowing graphical configuration of preferred media player (Auto, MPV, VLC, or custom path), download destination directory, YouTube cookies browser extraction (`chrome`, `firefox`, `edge`, `brave`), and active file locations.
  - Added one-click "Sign Out & Disconnect Account" button that safely purges token cache and resets view state to Login.
- **Media Download Progress Bar & Dependency Guidance (`youtube-gui`)**:
  - Added real-time percentage parsing (`parse_progress_percentage`) and smooth `egui::ProgressBar` rendering in both feed cards and Video Details view.
  - Added actionable error detection and tooltips when `yt-dlp` is missing with setup instructions (`winget install yt-dlp` / `brew install yt-dlp`).
- **Windows Installer & Release Packaging (`dist/windows/setup.iss`)**:
  - Added complete Inno Setup Windows installer script compiling standalone `youtube-client-setup-<version>.exe` with desktop shortcut task and PATH registration.
  - Updated `.github/workflows/release.yml` to automatically build and attach Windows setup executables alongside portable zip packages.
- **Return YouTube Dislike (RYD) Public API Integration**:
  - Integrated public `returnyoutubedislikeapi.com` API in `youtube-client-lib` with non-blocking async lookup and 3s connection timeout fallback.
  - Added `dislike_count` field to `VideoDetails` model across `youtube-client-lib`, mock testing fixtures, CLI `details` command, and GUI video details panel.
  - Restored community dislike metrics and dislike count display with explanatory hover tooltip in the GUI.
- **Audio Dock Interactive Seek & Scrubber (`youtube-gui`)**:
  - Upgraded embedded Rodio audio player thread with precise timeline tracking (`position_secs`, `duration_secs`) and `Seek`/`Skip` command handling.
  - Added interactive playback scrubber slider in bottom audio dock allowing instant drag-and-drop timeline scrubbing.
  - Added `⏪ 10s` and `⏩ 10s` instant skip buttons and elapsed/total duration readout (`MM:SS / MM:SS`).
- **Persistent Window Geometry & Session Restoration (`youtube-gui` & `youtube-client-lib`)**:
  - Added `window_pos`, `window_size`, and `window_maximized` fields to `Config` with centralized `save_config`.
  - Automatically captures window client size, screen position, and maximized state during GUI interaction, with debounced background saving and instant flushing on close request or app drop.
  - Restores exact window coordinates, dimensions, and maximized state on startup with screen boundary and minimum size guardrails.
- **Persistent Playback Progress Tracking & Resume Playback (`youtube-gui` & `youtube-client-lib`)**:
  - Added `PlaybackProgress` tracking storing exact watched timestamp (`position_secs`, `duration_secs`, `updated_at`) per video ID in `%APPDATA%/youtube-client/playback_positions.json` (or `~/.config/youtube-client/playback_positions.json`) using atomic tempfile writes.
  - Embedded audio worker continuously syncs playback progress every 200ms with debounced auto-saving every 3s, instant saving on Pause, Stop, or application exit, and auto-removal upon completion.
  - Feed cards and Video Details view visually display watch progress via a red progress bar under thumbnails and a `⏱ Watched to MM:SS` badge.
  - Action buttons in both feeds and Video Details offer instant `▶ Resume (MM:SS)` or `↺ Start` controls for local audio, local video, and streaming via external players (`--start` for MPV and `--start-time` for VLC).
- **API Quota Budget Tracking & Management (`youtube-client-lib` & `youtube-gui`)**:
  - Added `QuotaTracker` accurately modeling YouTube Data API v3 unit costs (search = 100u, reads = 1u, writes = 50u) with daily UTC rollover and atomic JSON persistence to `%APPDATA%/youtube-client/api_quota.json`.
  - Automatically captures API usage in GUI client actions, logging consumption and warning against quota exhaustion.
  - Added real-time visual Quota Budget card in Settings view with a dynamic color gauge and breakdown of searches, read queries, and mutations.
- **Cross-Platform Subscription Interoperability (`youtube-client-lib` & `youtube-gui`)**:
  - Added bidirectional export and import utilities for standard OPML XML (compatible with FreeTube, NewPipe, Feedly, and RSS readers), Google Takeout CSV (`subscriptions.csv`), and NewPipe backup JSON.
  - Added one-click "Export Subscriptions to OPML" in the Settings view saving directly to the user's downloads directory.
  - Added "Import Subscriptions (OPML / CSV)" in the Settings view with automatic format detection and deduplicating channel merge into active subscriptions.
- **In-Memory Metadata & Channel Cache with TTL (`youtube-client-lib`)**:
  - Added thread-safe `MetadataCache` and `TtlCache<K, V>` providing configurable time-to-live expiration for video metadata (30m) and channel summaries (2h), slashing redundant API calls and conserving quota.
  - Integrated `MetadataCache` and `QuotaTracker` directly into `YoutubeClient` and `YoutubeClientBuilder`, enabling transparent cache lookups and automatic cost accounting.
- **Zero-Quota Public Channel RSS Feed Fallback (`youtube-client-lib` & `youtube-gui`)**:
  - Added `fetch_channel_videos_via_rss` and `parse_youtube_rss_xml` parsing YouTube's public Atom/RSS XML feeds for channel uploads with zero API keys and 0 quota units.
  - Automatic fallback in GUI channel browsing if API requests fail or quota limits are reached.
- **CI Hygiene & Build Reproducibility**:
  - Resolved clippy dead code error in `youtube-installer/src/prompt.rs`.
  - Fixed code formatting in `youtube-installer/src/platform.rs`.
  - Committed `Cargo.lock` to ensure deterministic builds across platforms.

## [0.1.3] - Professional Product Polish, Standalone Installer & Release Automation

### Added
- **Multi-Platform CI/CD & GitHub Release Automation**:
  - Added `.github/workflows/ci.yml` matrix checking `ubuntu-latest`, `windows-latest`, and `macos-latest` with `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`.
  - Added `.github/workflows/release.yml` automating binary packaging, SHA256 checksum generation, and GitHub Release deployment on `v*` tag pushes.
- **Standalone Installer & Windows System Integration (`youtube-installer`)**:
  - Pre-built binary detection allowing non-developer end users to install immediately from downloaded zip releases without requiring `cargo` or Rust.
  - Windows "Add/Remove Programs" registration in `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\YouTubeClient` with custom icon, publisher, version, and clean uninstaller string.
  - Local installation of uninstaller utility (`youtube-installer.exe`) in installation directory.
- **CLI Shell Auto-Completions (`youtube-client`)**:
  - Added `completions` subcommand supporting `bash`, `zsh`, `fish`, `powershell`, and `elvish` via `clap_complete`.
- **Desktop GUI UX Polish (`youtube-gui`)**:
  - Non-intrusive in-app toast notification banner providing real-time feedback for media streaming, player fallbacks, downloads, and user actions.
  - Persistent volume configuration and muted state.
  - Complete elimination of console window and console trace leakage.
- **Code Quality & Ergonomics**:
  - Resolved all 85+ Clippy warnings across the entire workspace, reaching zero-warning strict hygiene with `-D warnings`.
  - Boxed large error variants in `YoutubeError` to reduce enum size and optimize `Result` stack usage.

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
