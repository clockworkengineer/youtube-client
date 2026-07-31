# DRY Refactor Plan: Phase 3 (UI Cards & Async Helpers consolidation)

This document details identified duplicate patterns introduced during Phase 3 & 4 and describes a concrete refactoring plan to enforce DRY.

---

## 1. Identified Duplication

### A. Triplicated Video Card Rendering
* **Location**: `youtube-gui/src/main.rs` (under match arms for `View::ChannelVideos`, `View::SearchResults`, and `View::PlaylistVideos`)
* **Problem**: Over 100 lines of UI layout logic for rendering a single video card (including thumbnail caching, title/metadata display, download status spinner, local playback triggers, and click handlers) are copied verbatim three times.
* **Refactor Plan**:
  * Extract this into a reusable helper method on `YoutubeGuiApp`:
    ```rust
    fn draw_video_card(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        video: &youtube_client_lib::Video,
        action: &mut PendingAction,
    )
    ```
  * Replace the inline code blocks in all three view match arms with calls to this helper.

### B. Repetitive Background Worker Spawning
* **Location**: `youtube-gui/src/main.rs`
* **Problem**: Background tasks like `spawn_subscribe`, `spawn_unsubscribe`, `spawn_rate_video`, and `spawn_add_to_playlist` have identical boilerplate for launching threads, getting async clients, executing calls, printing results, and refreshing states.
* **Refactor Plan**:
  * Unify state update signals by routing them through a single command channel or consolidate the runner logic into an associated method:
    ```rust
    fn run_interactive_task<F, Fut>(state: Arc<Mutex<AppState>>, ctx: egui::Context, task: F)
    ```

---

## 2. Refactoring Tasks

1. Create the `draw_video_card` layout helper in `youtube-gui/src/main.rs`.
2. Replace duplicate card code blocks in `ChannelVideos`, `SearchResults`, and `PlaylistVideos` render sections.
3. Clean up background task definitions.
