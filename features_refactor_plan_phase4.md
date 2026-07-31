# Feature Expansion Refactor Plan - Phase 4 (Interactive Management & Video Details)

This plan outlines the next phase of refactoring and feature additions to support interactive subscription/playlist modifications and advanced video details in the client.

---

## 1. Proposed Features

### A. Subscriptions Management
* **Ability**: Subscribe or unsubscribe to channels directly from search results or channel pages.
* **API Additions**:
  * `subscribe_to_channel(&self, channel_id: &str) -> anyhow::Result<()>`
  * `unsubscribe_from_channel(&self, subscription_id: &str) -> anyhow::Result<()>`
* **GUI Integration**: Add a "+ Subscribe" / "✓ Subscribed" toggle button on video channel views and search results.

### B. Playlist Modifications
* **Ability**: Add/remove videos to/from user playlists, and create new playlists.
* **API Additions**:
  * `create_playlist(&self, title: &str, description: Option<&str>) -> anyhow::Result<Playlist>`
  * `add_to_playlist(&self, playlist_id: &str, video_id: &str) -> anyhow::Result<()>`
  * `remove_from_playlist(&self, playlist_item_id: &str) -> anyhow::Result<()>`
* **GUI Integration**: Add an "Add to Playlist" dropdown button/menu to video cards.

### C. Video Rating & Details
* **Ability**: Like/dislike videos and fetch comments/related videos.
* **API Additions**:
  * `rate_video(&self, video_id: &str, rating: &str) -> anyhow::Result<()>` (rating = "like", "dislike", or "none")
  * `fetch_comments(&self, video_id: &str) -> anyhow::Result<Vec<Comment>>`
* **GUI Integration**: Like/Dislike thumbs buttons, and a collapsible "Comments" section in the video details panel.

---

## 2. Refactoring Tasks

1. **Step 1**: Implement library methods for subscription, playlist item additions, and ratings in `youtube-client-lib/src/lib.rs`.
2. **Step 2**: Add UI buttons and actions in `youtube-gui/src/main.rs` to allow subscription state toggling and video ratings.
3. **Step 3**: Implement "Add to Playlist" popover dialog or menu button in GUI cards.
