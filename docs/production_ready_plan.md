# Production-Ready Plan for youtube-client-lib

This document outlines the concrete plan and the architectural improvements applied to `youtube-client-lib` to make it production-ready.

## Goals & Objectives

1. **Robust Error Handling**: Replace generic `anyhow::Result` with a domain-specific `YoutubeError` enum using the `thiserror` crate to allow client applications (CLI/GUI) to handle distinct failure modes programmatically.
2. **API Rate Limiting & Retries**: Add support for transient failure recovery (exponential backoff and retries) when making external requests to Google's YouTube APIs.
3. **Async Playback Safety**: Ensure that playing audio via Rodio does not block the Tokio runtime threads.
4. **Environment & Dependency Validation**: Improve feedback when external binaries (like `yt-dlp`) are missing.

---

## Technical Details

### 1. Domain-Specific Errors (`thiserror`)
We define `YoutubeError` in the crate root (`lib.rs`):
- `Credentials(String)`: Issues loading client secrets or invalid placeholders.
- `Auth(yup_oauth2::Error)`: Authentication flow or token persistence failures.
- `Api(google_youtube3::Error)`: Direct API failures from the YouTube data service.
- `Io(std::io::Error)`: Filesystem and process execution issues.
- `Json(serde_json::Error)`: Serialization/deserialization errors.
- `Media(String)`: Local player launch or audio stream initialization failures.
- `Download(String)`: Failures during media extraction or downloader process runs.
- `YtDlpMissing(String)`: Explicit error when the `yt-dlp` executable cannot be located in the path.

### 2. Transient Error Retries
A retry handler `retry_api_call` wraps the client builders:
- Limits attempts to 3.
- Implements exponential backoff (starting at 500ms).
- Identifies retryable errors such as hyper connection failures, HTTP `429 (Too Many Requests)`, and `5xx` server-side responses.

### 3. Non-blocking Task Spawning for Media
The `play_audio_rodio` blocking loop is run in a separate OS threadpool using `tokio::task::spawn_blocking`. This keeps the async runtime responsive for other tasks (like UI rendering or concurrent downloads).

---

## Verification Plan

### Automated Tests
- Test that non-retryable errors (e.g., `MissingAPIKey`) fail immediately without retrying.
- Verify standard credential resolution and token cache validation flows.

### Manual Verification
- Test compile compatibility across both the CLI (`youtube-client`) and GUI (`youtube-gui`) applications.
- Run interactive downloads and check media playback controls.
