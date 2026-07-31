# DRY Refactor Plan: Phase 4 (Async Operations Consolidation)

This document details identified duplicate patterns introduced during Phase 4 and provides a concrete refactoring plan to consolidate them.

---

## 1. Identified Duplication

### A. Duplicate Spawning for Simple API Actions
* **Location**: `youtube-gui/src/main.rs` (under `spawn_subscribe`, `spawn_unsubscribe`, and `spawn_rate_video`)
* **Problem**: Each of these methods contains duplicate boilerplate for obtaining the async client, initiating a spawn task, executing the client future, catching errors, logging, and optionally invoking UI refreshes.
* **Refactor Plan**:
  * Implement a generic helper method on `YoutubeGuiApp`:
    ```rust
    fn spawn_client_action<F, Fut, T>(
        state: Arc<Mutex<AppState>>,
        ctx: egui::Context,
        action_name: &'static str,
        action_future: F,
        on_success: impl FnOnce(T, &mut AppState, &egui::Context) + Send + 'static,
    )
    where
        F: FnOnce(YoutubeClient) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
        T: Send + 'static;
    ```
  * Replace the individual boilerplates in `spawn_subscribe`, `spawn_unsubscribe`, and `spawn_rate_video` with calls to this helper.

---

## 2. Refactoring Tasks

1. Define `spawn_client_action` helper in `youtube-gui/src/main.rs`.
2. Rewrite `spawn_subscribe` and `spawn_unsubscribe` using the new generic worker helper.
3. Rewrite `spawn_rate_video` and `spawn_add_to_playlist` using the new helper.
