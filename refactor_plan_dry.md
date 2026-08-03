# DRY Refactoring Plan Phase 2 for youtube-client

Following the completion of the first set of refactoring steps, we have identified several remaining code duplication patterns (DRY violations) across `youtube-client`, `youtube-gui`, and `youtube-client-lib`.

---

## 1. Identified Duplications (DRY Violations)

### A. YouTube OAuth Scopes Definition
- **Location**:
  - `youtube-gui/src/main.rs` (lines 304–308) inside `get_client_async`
  - `youtube-client/src/main.rs` (lines 119–123) inside `get_client`
- **Duplication**: The list of YouTube OAuth scope URLs is defined word-for-word in both binary crates.
- **Proposed Solution**: Centralize this in `youtube-client-lib/src/lib.rs` as a public constant:
  ```rust
  pub const YOUTUBE_SCOPES: &[&str] = &[
      "https://www.googleapis.com/auth/youtube",
      "https://www.googleapis.com/auth/youtube.force-ssl",
      "https://www.googleapis.com/auth/youtube.readonly",
  ];
  ```
  And update both CLI and GUI crates to reference `youtube_client_lib::YOUTUBE_SCOPES`.

### B. Token Cache Scope Verification
- **Location**: `youtube-gui/src/main.rs` (lines 263–293) inside `get_client_async`.
- **Duplication**: The complex logic that parses `tokencache.json` to verify if the cache contains the full YouTube scopes is currently placed directly inside the GUI crate. This verification logic is generally useful for any client (including a CLI client) to check if a cached token needs refreshing.
- **Proposed Solution**: Extract this into a library helper function in `youtube-client-lib/src/lib.rs`:
  ```rust
  pub fn check_token_cache_scopes(token_cache_path: &Path, required_scopes: &[&str]) -> bool
  ```
  This keeps the JSON parsing and validation logic inside the library and simplifies the GUI's `get_client_async`.

### C. Direct `yt-dlp` Process Invocation
- **Location**:
  - `youtube-client/src/main.rs` (lines 167–178) inside `Commands::Download` fallback.
  - `youtube-client-lib/src/lib.rs` (lines 518–535) inside `download_video`.
- **Duplication**: Both locations manually construct and spawn a `tokio::process::Command` calling `yt-dlp` with the same arguments (format selection, MP3 conversion parameters, and output path template).
- **Proposed Solution**: Refactor the download implementation in `youtube-client-lib` to expose a standalone public function or associated static method:
  ```rust
  pub async fn download_video_direct<F>(video_id: &str, output_path: &Path, on_progress: F) -> anyhow::Result<()>
  where
      F: Fn(&str) + Send + Sync + 'static;
  ```
  Both the library's `download_video(&self, ...)` and the CLI client's fallback download can call this common implementation.

### D. Config File Loading & Fallbacks
- **Location**: `youtube-client/src/main.rs` (lines 79–87) inside `get_credentials`.
- **Duplication**: Manually reading the custom config path or falling back to `load_config()`.
- **Proposed Solution**: Add a helper function to `youtube-client-lib/src/lib.rs` that loads a config from a custom path, with default fallback:
  ```rust
  pub fn load_config_from_file_or_default(path: &Path) -> Config
  ```

---

## 2. Refactoring Execution Checklist

- [x] **Step 1: Centralize OAuth Scopes**
  - Define `YOUTUBE_SCOPES` constant in `youtube-client-lib/src/lib.rs`.
  - Update `youtube-gui` and `youtube-client` to use `YOUTUBE_SCOPES`.
- [ ] **Step 2: Extract Token Cache Verification Helper**
  - Implement `check_token_cache_scopes` in `youtube-client-lib/src/lib.rs`.
  - Simplify the verification block in `youtube-gui/src/main.rs`.
- [ ] **Step 3: Centralize direct `yt-dlp` download logic**
  - Implement `download_video_direct` in `youtube-client-lib/src/lib.rs`.
  - Refactor `download_video` in `youtube-client-lib` and `Commands::Download` in `youtube-client` to use the new library method.
- [ ] **Step 4: Refactor Config Loader**
  - Implement `load_config_from_file_or_default` in `youtube-client-lib/src/lib.rs`.
  - Replace the custom config file parsing logic in `youtube-client/src/main.rs`.
