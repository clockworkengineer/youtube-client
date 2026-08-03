# DRY Refactoring Plan for youtube-client

During our recent work, we identified several patterns of code duplication (DRY violations) across `youtube-gui` and `youtube-client-lib`. Below is the concrete refactoring plan to streamline the codebase, reduce boilerplate, and make maintenance easier.

---

## 1. Identified Duplications (DRY Violations)

### A. Media Player Spawning
- **Location**: `youtube-gui/src/main.rs` inside `PendingAction::StreamVideo` and `PendingAction::PlayLocal`.
- **Duplication**: The list of fallback player paths (`mpv`, `vlc`, standard installation paths on Windows) and the loop to spawn the process is repeated word-for-word in both match arms.
- **Proposed Solution**: Extract a helper function:
  ```rust
  fn launch_media_player(target: &std::ffi::OsStr) -> Result<(), String>
  ```
  This helper will retrieve the configured player path, append the fallback search list, and attempt to spawn the command, returning `Ok(())` on the first success or an error if all fail.

### B. Background API Fetching Boilerplate
- **Location**: `youtube-gui/src/main.rs` in `fetch_videos`, `fetch_search_results`, `fetch_playlist_videos`, `spawn_fetch_comments`, and `spawn_fetch_playlists`.
- **Duplication**: Every single fetch method duplicates:
  1. Spawning a tokio task (`tokio::spawn(async move { ... })`).
  2. Acquiring the async client (`let client = Self::get_client_async().await?;`).
  3. Wrapping the call in an `async` block to catch errors.
  4. Locking `state` and mapping the result to updating a specific `View` variant.
  5. Calling `ctx.request_repaint()`.
- **Proposed Solution**: Implement a generic state-update runner:
  ```rust
  fn spawn_fetch_action<F, Fut, T>(
      state: Arc<Mutex<AppState>>,
      ctx: egui::Context,
      f: F,
      on_success: impl FnOnce(T, &mut AppState) + Send + 'static,
      on_failure: impl FnOnce(String, &mut AppState) + Send + 'static,
  )
  where
      F: FnOnce(YoutubeClient) -> Fut + Send + 'static,
      Fut: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
      T: Send + 'static;
  ```
  This collapses the repetitive spawning boilerplate down to just the unique API call and the state assignment logic.

### C. File Naming and Output Path Construction
- **Location**: `youtube-gui/src/main.rs` inside `spawn_download` and `extract_video_id_from_path` logic.
- **Duplication**: Constructing the target path `[downloads_dir]/[channel_title]/[video_title] [[video_id]].ext` involves path resolution and sanitization which could be centralized.
- **Proposed Solution**: Consolidate output path generation into a single helper on `AppState` or as a standalone function:
  ```rust
  fn get_download_path(downloads_base: &Path, channel_title: &str, video_title: &str, video_id: &str, is_audio: bool) -> PathBuf
  ```

---

## 2. Refactoring Execution Checklist

- [x] **Step 1: Extract Media Player Spawning**
  - Implement `launch_media_player(target: &std::ffi::OsStr) -> Result<(), String>`.
  - Replace the player spawning blocks in `PendingAction::StreamVideo` and `PendingAction::PlayLocal`.
- [x] **Step 2: Consolidate Background Fetch Boilerplate**
  - Implement the generic `spawn_fetch_action` wrapper.
  - Refactor `fetch_videos`, `fetch_search_results`, `fetch_playlist_videos`, `spawn_fetch_comments`, and `spawn_fetch_playlists` to utilize the new wrapper.
- [x] **Step 3: Centralize Download Path Logic**
  - Extract `get_download_path` helper.
  - Simplify path building in `spawn_download`.
