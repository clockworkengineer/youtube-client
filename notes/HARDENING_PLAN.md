# Comprehensive Codebase Hardening Plan (`youtube-client` Workspace)

## 1. Executive Summary & Audit Scope

A deep security, reliability, concurrency, and resource-safety audit was conducted across all four workspace crates (`youtube-client-lib`, `youtube-client`, `youtube-gui`, and `youtube-installer`). While the workspace already features strong foundational practices (atomic sibling file writes via `NamedTempFile`, `0600` Unix token permissions, and Windows reserved-filename sanitization), the audit identified **6 high-impact hardening domains** that should be addressed to make the system production-hardened against panics, injection attacks, resource exhaustion, and hanging I/O.

---

## 2. Summary of Identified Vulnerabilities & Fragilities

| ID | Category | Severity | Location(s) | Issue Summary |
| :---: | :--- | :---: | :--- | :--- |
| **H1** | **Input Validation & Path/URL Injection** | **High** | [`download.rs`](../youtube-client-lib/src/download.rs), [`audio.rs`](../youtube-client-lib/src/audio.rs), [`sponsorblock.rs`](../youtube-client-lib/src/sponsorblock.rs), [`rss.rs`](../youtube-client-lib/src/rss.rs), [`playback.rs`](../youtube-gui/src/handlers/playback.rs), [`mpv_ipc.rs`](../youtube-client-lib/src/mpv_ipc.rs) | `video_id`, `channel_id`, and `instance_id` are interpolated directly into URLs and `%TEMP%/youtube-client-audio/{vid}.mp3` paths without character allowlist validation (`[A-Za-z0-9_-]`), allowing potential path traversal (`../`) or URL query parameter injection. |
| **H2** | **Subprocess & Shell Argument Hardening** | **High** | [`player.rs`](../youtube-client-lib/src/player.rs) | `SystemDefaultPlayer` on Windows invokes `cmd /C start "" <target>`, exposing shell metacharacters (`&`, `|`, `^`). External players (`MpvPlayer`, `VlcPlayer`, `IinaPlayer`, `CustomExecutablePlayer`) do not pass `--` before `target`, allowing leading-dash filenames to be parsed as player flags. |
| **H3** | **Credential Storage & Plaintext CWD Leak** | **High** | [`actions.rs`](../youtube-gui/src/actions.rs), [`config_writer.rs`](../youtube-installer/src/config_writer.rs) | `spawn_login_and_auth` writes `private_config.json` (containing `client_secret`) non-atomically to the current working directory (`std::fs::write("private_config.json", ...)`) without `secure_sensitive_file` (`0600`) permissions. |
| **H4** | **Panic Elimination & Poisoned Lock Recovery** | **Medium** | [`actions.rs`](../youtube-gui/src/actions.rs), [`playback.rs`](../youtube-gui/src/handlers/playback.rs), [`cache.rs`](../youtube-client-lib/src/cache.rs), [`quota.rs`](../youtube-client-lib/src/quota.rs), [`sponsorblock.rs`](../youtube-client-lib/src/sponsorblock.rs), [`utils.rs`](../youtube-client-lib/src/utils.rs) | 15+ raw `.lock().unwrap()` / `.read().unwrap()` / `.write().unwrap()` calls across GUI and library caches; `partial_cmp().unwrap()` on `f64` timestamps in `sponsorblock.rs` (panics on `NaN`); unchecked slice indexing `widths[i]` in `format_table`. |
| **H5** | **Memory, Disk & Recursion Bounds (DoS / OOM)** | **Medium** | [`utils.rs`](../youtube-client-lib/src/utils.rs), [`audio.rs`](../youtube-client-lib/src/audio.rs), [`download.rs`](../youtube-client-lib/src/download.rs), [`playback.rs`](../youtube-gui/src/handlers/playback.rs), [`actions.rs`](../youtube-gui/src/actions.rs) | Recursive directory scanners (`scan_downloads_dir`, `scan_downloaded_items`) follow symlinks with no depth cap; `stream_audio_rodio` buffers stdout into RAM without a max byte cap; `yt-dlp` stderr/stdout buffers have no cap; `%TEMP%/youtube-client-audio` never evicts old cached audio files. |
| **H6** | **Network & IPC Timeouts** | **Medium** | [`actions.rs`](../youtube-gui/src/actions.rs), [`playback.rs`](../youtube-gui/src/handlers/playback.rs), [`sponsorblock.rs`](../youtube-client-lib/src/sponsorblock.rs), [`rss.rs`](../youtube-client-lib/src/rss.rs), [`mpv_ipc.rs`](../youtube-client-lib/src/mpv_ipc.rs) | Ad-hoc `reqwest::Client::new()` instances created without connect/read timeouts; `MpvIpcClient` socket reads (`read_line`) have no timeout or max line length cap (`take`), risking indefinite thread hangs. |

---

## 3. Phased Hardening Implementation Plan

### Phase 1: Input Validation, Path Traversal & Subprocess Hardening (H1, H2, H3)

#### 1.1. Strict Identifier Validation in [`youtube-client-lib/src/utils.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/utils.rs)
* Add reusable, zero-allocation validator functions:
  - `pub fn is_valid_video_id(id: &str) -> bool`: Verifies `id.len() >= 8 && id.len() <= 32` and `id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')`.
  - `pub fn is_valid_channel_id(id: &str) -> bool`: Verifies `!id.is_empty() && id.len() <= 64` and `id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '@')`.
  - `pub fn sanitize_identifier(id: &str) -> String`: Strips any character outside `[A-Za-z0-9_-]` for safe use in cache filenames and IPC socket names.
* **Enforce across entry points:**
  - [`youtube-client-lib/src/download.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/download.rs#L500-L515) (`download_video_with_options`): Reject invalid `video_id` values with `YoutubeError::InvalidInput` before constructing URLs or spawning `yt-dlp`.
  - [`youtube-client-lib/src/audio.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/audio.rs#L43-L55) (`stream_audio_rodio`): Reject invalid `video_id` values with `YoutubeError::InvalidInput`.
  - [`youtube-client-lib/src/sponsorblock.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/sponsorblock.rs#L80-L92) (`fetch_skip_segments`): Validate `video_id` before interpolating into `https://sponsor.ajay.app/api/skipSegments`.
  - [`youtube-client-lib/src/rss.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/rss.rs#L234-L240) (`fetch_channel_videos_via_rss`): Validate `channel_id` before constructing the feed URL.
  - [`youtube-gui/src/handlers/playback.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/playback.rs#L222-L228) (`StreamAudioDirect`): Sanitize `vid` via `sanitize_identifier(&vid)` before constructing `cache_dir.join(format!("{safe_vid}.mp3"))` so path traversal out of `%TEMP%/youtube-client-audio` is impossible.
  - [`youtube-client-lib/src/mpv_ipc.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/mpv_ipc.rs#L24-L33) (`MpvIpcClient::generate_pipe_path`): Sanitize `instance_id` via `sanitize_identifier` so `/tmp/mpv-ipc-{id}.sock` cannot traverse directories.

#### 1.2. Subprocess & External Player Hardening in [`youtube-client-lib/src/player.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs)
* **Eliminate `cmd /C start` shell invocation in `SystemDefaultPlayer::launch` ([`player.rs:444-452`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs#L444-L452)):**
  - Validate that `target` is either a valid `http://` / `https://` URL or an existing local filesystem path (`Path::new(target).exists()`), and launch via `rundll32.exe url.dll,FileProtocolHandler <target>` on Windows (or `open` / `xdg-open` with `--` on macOS/Linux) to completely avoid `cmd.exe` metacharacter interpretation (`&`, `|`, `>`, `<`).
* **Add end-of-options `--` separator to all media players ([`player.rs:179-424`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs#L179-L424)):**
  - In `MpvPlayer::launch`, `VlcPlayer::launch`, `IinaPlayer::launch`, and `CustomExecutablePlayer::launch`, change `cmd.arg(target)` to `cmd.arg("--").arg(target)` so filenames or targets starting with `-` cannot inject player flags (such as `--script=` in MPV).

#### 1.3. Secure Credential Persistence in [`youtube-gui/src/actions.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs) & [`youtube-installer/src/config_writer.rs`](file:///c:/Projects/youtube-client/youtube-installer/src/config_writer.rs)
* In [`youtube-gui/src/actions.rs:195-207`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs#L195-L207) (`spawn_login_and_auth`):
  - Replace `std::fs::write("private_config.json", content)` with atomic saving into the user's global configuration directory (`youtube_client_lib::config::resolve_app_data_path("config.json")`) preserving existing `Config` fields (`load_config` -> update `client_id` & `client_secret` -> `save_config`), and call `youtube_client_lib::config::secure_sensitive_file` (`0600` on Unix).
* In [`youtube-installer/src/config_writer.rs`](file:///c:/Projects/youtube-client/youtube-installer/src/config_writer.rs):
  - Write `config.json` atomically and apply `0600` permissions on Unix when `client_secret` is written.

---

### Phase 2: Panic Elimination & Poisoned Lock Recovery (H4)

#### 2.1. Replace Raw `.unwrap()` on Locks in `youtube-gui` and `youtube-client-lib`
* **[`youtube-gui/src/actions.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs):**
  - Replace all 14 occurrences of `state.lock().unwrap()` / `state_clone.lock().unwrap()` / `sb_state.lock().unwrap()` with `crate::types::lock_state(&state)` (which gracefully recovers from poisoned mutexes via `.unwrap_or_else(|e| e.into_inner())`).
* **[`youtube-gui/src/handlers/playback.rs:32`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/playback.rs#L32):**
  - Replace `state.lock().unwrap()` with `lock_state(state)`.
* **[`youtube-client-lib/src/cache.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/cache.rs):**
  - Add private helper methods `read_entries(&self)` and `write_entries(&self)` on `TtlCache` using `.unwrap_or_else(|e| e.into_inner())`, replacing all 8 `.read().unwrap()` and `.write().unwrap()` calls.
* **[`youtube-client-lib/src/quota.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/quota.rs):**
  - Add a private `lock_status(&self)` helper using `.unwrap_or_else(|e| e.into_inner())`, replacing all 3 `self.status.lock().unwrap()` calls.

#### 2.2. Float `NaN` & Slice Bounds Safety
* **[`youtube-client-lib/src/sponsorblock.rs:132-160`](file:///c:/Projects/youtube-client/youtube-client-lib/src/sponsorblock.rs#L132-L160) (`build_mpv_edl`):**
  - Reject non-finite `total_duration_secs` (`!total_duration_secs.is_finite() || total_duration_secs <= 0.0`).
  - Filter `skip_segments` to retain only segments where `seg.start_secs.is_finite() && seg.end_secs.is_finite() && seg.start_secs >= 0.0 && seg.end_secs > seg.start_secs`.
  - Replace `a.start_secs.partial_cmp(&b.start_secs).unwrap()` with `a.start_secs.total_cmp(&b.start_secs)`.
* **[`youtube-client-lib/src/utils.rs:199-227`](file:///c:/Projects/youtube-client/youtube-client-lib/src/utils.rs#L199-L227) (`format_table`):**
  - Replace unchecked `widths[i]` indexing with `widths.get(i).copied().unwrap_or(12)` and `saturating_sub` so mismatched header/column/width slices never panic.
* **[`youtube-installer/src/main.rs:234,254`](file:///c:/Projects/youtube-client/youtube-installer/src/main.rs#L234-L254):**
  - Replace `cli_src.file_name().unwrap()` and `gui_src.file_name().unwrap()` with `.and_then(|p| p.file_name()).ok_or_else(...)`.

---

### Phase 3: Resource Bounds, Symlink Protection & Cache Eviction (H5)

#### 3.1. Bounded Directory Traversal Without Following Symlinks ([`youtube-client-lib/src/utils.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/utils.rs#L97-L183))
* In `scan_downloads_dir` and `scan_downloaded_items_inner`:
  - Add a `max_depth: usize` parameter (capped at `MAX_SCAN_DEPTH = 4`) and a maximum file count cap (`MAX_SCANNED_FILES = 5,000`).
  - Check `entry.file_type()` and skip symlinks (`!ft.is_symlink() && ft.is_dir()`) so circular directory symlinks cannot cause infinite recursion or stack overflow.

#### 3.2. Memory Caps on Subprocess Streams & HTTP Responses
* **[`youtube-client-lib/src/audio.rs:75-100`](file:///c:/Projects/youtube-client/youtube-client-lib/src/audio.rs#L75-L100) (`stream_audio_rodio`):**
  - Enforce `const MAX_AUDIO_STREAM_BYTES: usize = 256 * 1024 * 1024;` (256 MB). If `audio_bytes.len() + n > MAX_AUDIO_STREAM_BYTES`, kill the child process and return `Err(YoutubeError::Media("Audio stream exceeded maximum 256 MB in-memory buffer limit".into()))`.
* **[`youtube-client-lib/src/download.rs:617-696`](file:///c:/Projects/youtube-client/youtube-client-lib/src/download.rs#L617-L696) (`download_video_with_options`):**
  - Cap `err_buf` at `256 * 1024` bytes (keeping the most recent error output) and cap `line_buf` / `buffer` at `64 * 1024` bytes before draining so pathological output without newlines cannot exhaust memory.
* **[`youtube-gui/src/actions.rs:1034-1060`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs#L1034-L1060) (`fetch_thumbnail`):**
  - Enforce a `MAX_THUMBNAIL_BYTES: usize = 10 * 1024 * 1024` (10 MB) cap before decoding images in memory.

#### 3.3. Automatic Temp Audio Cache Pruning ([`youtube-gui/src/handlers/playback.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/playback.rs#L222-L230))
* Add `prune_temp_audio_cache(cache_dir: &Path, max_files: usize)` (e.g., `max_files = 25`):
  - Before extracting a new audio stream into `%TEMP%/youtube-client-audio`, inspect existing `.mp3` / `.m4a` files in `cache_dir`; if count exceeds `25`, remove the oldest files by modification time (`modified()`) and delete any leftover `0`-byte or `.part` files.

---

### Phase 4: Network Timeouts, Connection Reuse & IPC Hardening (H6)

#### 4.1. Standardized Timeout-Configured HTTP Client
* Add a helper `youtube_client_lib::utils::default_http_client() -> reqwest::Client` configured with:
  - `.connect_timeout(Duration::from_secs(5))`
  - `.timeout(Duration::from_secs(15))`
* Use this timeout-configured client (or `AppState::http_client` in `youtube-gui`) instead of bare `reqwest::Client::new()` in:
  - [`youtube-gui/src/actions.rs:450`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs#L450) (RSS fallback)
  - [`youtube-gui/src/actions.rs:602`](file:///c:/Projects/youtube-client/youtube-gui/src/actions.rs#L602) (SponsorBlock fetch)
  - [`youtube-gui/src/handlers/playback.rs:117`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/playback.rs#L117) (SponsorBlock MPV EDL check)
  - [`youtube-gui/src/main.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/main.rs) (`AppState::http_client` initialization)

#### 4.2. MPV IPC Socket Timeouts & Bounded Reads ([`youtube-client-lib/src/mpv_ipc.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/mpv_ipc.rs#L41-L94))
* On Unix (`UnixStream`), set `stream.set_read_timeout(Some(Duration::from_secs(3)))` and `stream.set_write_timeout(Some(Duration::from_secs(3)))`.
* On both Windows and Unix, wrap the reader with `.take(64 * 1024)` before calling `read_line(&mut resp_line)` so a misbehaving or stalled pipe cannot allocate unbounded memory.

---

## 4. Verification & Regression Test Plan

Each phase will be accompanied by dedicated unit tests:
1. **`youtube-client-lib` Hardening Tests:**
   - `test_validate_video_and_channel_ids`: Verify rejection of `../`, query strings (`?a=b`), whitespace, and shell metacharacters.
   - `test_sponsorblock_edl_nan_and_invalid_ranges`: Verify `build_mpv_edl` never panics on `f64::NAN`, `f64::INFINITY`, negative timestamps, or inverted ranges (`start > end`).
   - `test_format_table_mismatched_widths`: Verify `format_table` never panics when `headers.len() > widths.len()`.
   - `test_scan_downloads_depth_bound`: Verify directory scanner respects `MAX_SCAN_DEPTH` and ignores symlinks.
   - `test_mpv_ipc_pipe_path_sanitization`: Verify `MpvIpcClient::generate_pipe_path("../evil/path")` strips path separators.
2. **Full Workspace Quality Gate:**
   - `cargo test --workspace` (all unit, integration, and doc tests passing).
   - `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings).
   - `cargo fmt --all -- --check`.

