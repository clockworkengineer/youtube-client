# Feature Expansion Refactor Plan - Phase 2 (Search, Playlists & Built-in Player)

This plan outlines the design and implementation steps to add search capabilities, playlist browsing, and an integrated audio player to `youtube-gui`.

---

## 1. New Features

### A. Global Search
* **Description**: Users should be able to search YouTube for videos using keywords.
* **Proposed Implementation**:
  * Add a `View::SearchResults { query: String, videos: Option<Result<Vec<Video>, String>> }` variant.
  * Render a search bar at the top of the header in both `View::Subscriptions` and `View::SearchResults`.
  * Use the library's YouTube API search capabilities (or add `search_videos` to `YoutubeClient` if missing).
  * Render the search results in a card list similar to the channel videos view.

### B. Built-in Audio Player (via Rodio)
* **Description**: Play audio from downloaded files directly inside the GUI application without launching external media players.
* **Proposed Implementation**:
  * Enable the `audio` feature on `youtube-client-lib` in `youtube-gui/Cargo.toml`.
  * Add a persistent playback control bar/dock at the bottom of the window (Play, Pause, Stop, Current Track Title).
  * Manage player state (active sink/stream) in `AppState` or a background thread.
  * When clicking "Play Local" on a downloaded video, play the audio track locally inside the GUI app.

### C. Playlists Browser
* **Description**: Let users browse and view videos from their personal playlists.
* **Proposed Implementation**:
  * Add a tab/navigation button to switch between "Subscriptions" and "Playlists".
  * Query user playlists using the YouTube Data API.

---

## 2. Step-by-Step Implementation

### Step 1: Add Search to `YoutubeClient`
If `YoutubeClient` doesn't have a search method, implement it in `youtube-client-lib/src/lib.rs`:
```rust
pub async fn search_videos(&self, query: &str, max_results: u32) -> anyhow::Result<Vec<Video>>
```

### Step 2: Implement Search UI & State
1. Update `View` enum in `youtube-gui/src/main.rs`.
2. Add search query input state and render the text input in the top header.
3. Query search results on Enter/Submit.

### Step 3: Implement Audio Player Integration
1. Enable `audio` feature in Cargo.toml.
2. Build playback UI dock and wire controls to the audio output device.
