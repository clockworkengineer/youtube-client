# YouTube Client Library Production Readiness Plan

This document outlines a concrete implementation plan to make the `youtube-client-lib` library crate robust, secure, maintainable, and fully production-ready.

## Overview & Architecture

Currently, `youtube-client-lib` is a helper library facilitating YouTube Data API v3 operations, video/audio downloads using `yt-dlp`, and audio playback using `rodio`. While it contains basic unit tests and utility implementations, it needs enhancements in error handling, API resilience, logging, diagnostics, asynchronous task isolation, and configuration modularity before being shipped in a production context.

---

## Proposed Changes & Improvement Plan

### 1. Robust API Integration & Resiliency
- **Paging Support**: Currently, operations like `list_subscriptions`, `list_videos`, `search_videos`, `list_playlists`, and `list_playlist_videos` have hard-coded page size limits (e.g., `.max_results(limit)`) capped at `50` and do not support paging tokens. We will add page token parameters (`page_token: Option<String>`) or implement an iterator/stream pattern to traverse paginated results.
- **Enhanced Retry Policies**: Refine `retry_api_call` to use exponential backoff with jitter instead of static wait periods, handling transient network dropouts and YouTube API rate limiting (`429 Too Many Requests`) elegantly.
- **Quota Exceeded Detection**: Expose specific, actionable error variants when Google API quota limits are hit, allowing consuming applications to gracefully display quota limit warnings rather than generic "API request failed" errors.

### 2. Modernized Download Interface
- **Remove `yt-dlp` Subprocess Dependency**: The crate's `download` feature currently declares `rusty_ytdl` in `Cargo.toml`, but `download_video_direct` shells out to the `yt-dlp` executable. We should fully leverage `rusty_ytdl` (which is already a dependency) to download streams in-process over HTTP, removing external process spawning dependencies unless fallback is specifically configured.
- **Download Lifecycle/State management**: Provide a robust async Stream/channel interface to emit progress updates (bytes downloaded, total bytes, transfer speed) instead of passing a text-based `Fn(&str)` callback.

### 3. Production-Grade Configuration & Credentials
- **Secure Storage Options**: Avoid storing credentials in plain JSON configuration files (`config.json`/`private_config.json`) by default. Integrate with platform keystores or provide extension points to load credentials from secure memory or secrets managers.
- **Scoped Cache Validation**: Fix the scope checking utility (`check_token_cache_scopes`) to ensure we validate all authorization boundaries before attempting calls, preventing token refresh loops or cryptic HTTP `403 Forbidden` API responses.

### 4. Non-Blocking Audio & Media Subsystems
- **Asynchronous Audio Engine**: The current audio playback function `play_audio_rodio` uses `tokio::task::spawn_blocking` and sleeps the thread until completion. We need a non-blocking playback handle that allows consumers to pause, resume, adjust volume, query current playback position, and stop playback dynamically.
- **Safe Process Spawning**: If any helper subprocesses (like external media players or browsers for OAuth) are invoked, ensure correct handling of stdin/stdout/stderr pipes, and avoid leaking zombie processes or hanging threads.

### 5. Diagnostics & Observability
- **Structured Logging**: Integrate the `tracing` or `log` crate. Currently, there is no instrumentation in the library, making troubleshooting in production environments difficult.
- **Contextual Error Reporting**: Add detailed context to errors (e.g., which request failed, which channel ID could not be loaded, etc.) using `thiserror`'s formatting features.

### 6. Code Quality, Safety, & Testing
- **API Documentation**: Add thorough documentation comments (`///`) on all public structures, methods, and error types, ensuring `#[warn(missing_docs)]` compiles clean.
- **Automated Mock Testing**: Implement mock HTTP responses for `google-youtube3` calls in tests, separating unit tests from real network/API access to enable fast, offline, and reliable CI checks.

---

## Verification Plan

### Automated Tests
- Introduce offline unit tests with mocked API endpoints.
- Validate token scope checking logic across various cache formats.
- Add integration tests for configuration overrides and credential loading hierarchies.

### Manual Verification
- Test audio playback pausing and stopping interfaces in the desktop GUI application.
- Verify download progress reporting correctness with varying network conditions.
- Test fallback behavior and instructions when client ID/secrets are missing.
