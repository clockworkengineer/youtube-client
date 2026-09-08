# Changelog

All notable changes to the `youtube-client` workspace are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Quality Attributes Architecture Refactor

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

### Changed
- **Modular Library Architecture**:
  - Decomposed the 994-line monolithic `youtube-client-lib/src/lib.rs` into focused modules: `client`, `builder`, `download`, `audio`, `retry`, `error`, `models`, `traits`, `utils`, `config`, and `testing`.
  - Converted `youtube-client-lib` default features to `[]` (opt-in), reducing compile times and unnecessary dependencies for headless consumers.
- **Removed**:
  - Dead dependency `rusty_ytdl = "0.7.4"` removed from workspace root and crates, eliminating redundant compilation overhead.
