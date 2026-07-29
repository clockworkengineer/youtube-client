# Feature Parity Refactor Plan: Adding Missing Features to GUI

This plan outlines the design and implementation steps to add missing core features to the GUI client (`youtube-gui`), bringing it to complete feature parity with the CLI client (`youtube-client`).

---

## 1. Missing Features

### A. GUI-driven Authentication / Login
* **Problem**: If `tokencache.json` or config file is missing, the GUI fails with a static error message requesting the user to run the CLI to login.
* **Proposed Implementation**:
  * Create a new `View::Login` state in `youtube-gui`.
  * If the credentials or token cache are missing, transition automatically to `View::Login`.
  * Display input fields for **Client ID** and **Client Secret**, along with a "Save & Authenticate" button.
  * Spawns an async tokio task that saves credentials to `private_config.json`, runs the `YoutubeClient::new_oauth` browser flow, saves the token, and transitions back to `View::Subscriptions`.

### B. Video Downloading from GUI
* **Problem**: The GUI only allows streaming videos via VLC/MPV/browser. It does not support downloading videos locally, which is supported in CLI/Library.
* **Proposed Implementation**:
  * Since GUI compiles `youtube-client-lib` with `default-features = false` (which strips `rusty_ytdl`), we will enable the `download` feature on `youtube-client-lib` for `youtube-gui`.
  * Add a "Download" button next to each video card in the Channel Videos view.
  * Maintain a map of active download tasks and progress in `AppState` (e.g. `downloads: HashMap<String, DownloadStatus>`).
  * When clicked, spawn a tokio task to call `client.download_video(video_id, path)` and update status to `Downloading`, `Completed(path)`, or `Failed(error)`.

### C. Local Media Playback
* **Problem**: Clicking a video card always attempts to stream. There is no way to play a downloaded local file.
* **Proposed Implementation**:
  * When a video is downloaded, change the download button to a "Play Local" button.
  * Clicking "Play Local" will open the local downloaded file using the configured system player (`open::that(file_path)`).

---

## 2. Step-by-Step Implementation Plan

### Step 1: Re-enable `download` feature for GUI
Update `youtube-gui/Cargo.toml` to include the `download` feature:
```toml
youtube-client-lib = { workspace = true, features = ["download"] }
```

### Step 2: Implement `View::Login` & Auth UI
1. Add `View::Login` variant to the `View` enum.
2. If `get_client_async` fails, set the view to `View::Login` instead of displaying a terminal error.
3. Render text input fields for credentials, and spawn the OAuth flow on button press.

### Step 3: Implement Download UI & State
1. Update `AppState` to track download tasks:
   ```rust
   #[derive(Clone)]
   enum DownloadStatus {
       NotStarted,
       Downloading,
       Finished(PathBuf),
       Failed(String),
   }
   ```
2. Render a "Download" button next to each video in the list showing its current status.
3. Spawn download tasks and trigger repaint upon completion.
