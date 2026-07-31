# Feature Expansion Refactor Plan - Phase 3 (Playlists Browser)

This plan details the steps to fetch and render the user's custom YouTube Playlists in the GUI.

---

## 1. Proposed Changes

### A. YouTube Client Library (`youtube-client-lib`)
* **New Struct**:
  ```rust
  #[derive(Clone, Debug)]
  pub struct Playlist {
      pub id: String,
      pub title: String,
      pub description: String,
      pub thumbnail_url: String,
      pub video_count: u32,
  }
  ```
* **New Methods** on `YoutubeClient`:
  1. `list_playlists(&self, max_results: u32) -> anyhow::Result<Vec<Playlist>>` to query all custom playlists created by the user.
  2. `list_playlist_videos(&self, playlist_id: &str, max_results: u32) -> anyhow::Result<Vec<Video>>` to retrieve videos inside a chosen playlist.

### B. GUI Client (`youtube-gui`)
* **Navigation Bar**: Add a side-by-side tab/navigation panel at the top header to switch between **Subscriptions** and **Playlists**.
* **New View States**:
  * `View::Playlists` to list all user playlists.
  * `View::PlaylistVideos` to list all videos within a specific playlist.
* **State Updates**:
  * Add `playlists: Option<Result<Vec<Playlist>, String>>` to `AppState`.
* **Background Tasks**:
  * Implement background fetching for user playlists and playlist items.

---

## 2. Implementation Steps

### Step 1: Implement Library Methods
Add `Playlist` struct, `list_playlists`, and `list_playlist_videos` in `youtube-client-lib/src/lib.rs`.

### Step 2: Implement GUI States & UI Navigation
Add playlist navigation buttons at the top of the GUI. Build the Playlists grid/list view and the playlist video detail viewer.
