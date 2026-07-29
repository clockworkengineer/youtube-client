# DRY Refactor Plan: Phase 2

This document details additional duplicate code patterns within the workspace and provides steps to refactor them.

---

## 1. Identified Duplication

### A. Repetitive CLI Client Construction
* **Location**: `youtube-client/src/main.rs` (under subcommands `Login`, `Subscriptions`, `Videos`, and `Download`)
* **Problem**: The following block is duplicated in multiple match arms:
  ```rust
  let (client_id, client_secret) = get_credentials()?;
  let client = YoutubeClient::new_oauth(&client_id, &client_secret, &cli.token_cache).await?;
  ```
* **Refactor Plan**:
  * Extract this into a helper function in `youtube-client/src/main.rs`:
    ```rust
    async fn get_client(cli: &Cli, credentials_helper: impl Fn() -> anyhow::Result<(String, String)>) -> anyhow::Result<YoutubeClient>
    ```
    or simply pass `client_id` and `client_secret` variables.

### B. Duplicate Subscription Fetch and GUI State Update Spawning
* **Location**: `youtube-gui/src/main.rs` (in `new` constructor and the `Retry` button click handler)
* **Problem**: The exact same `tokio::spawn` task is written twice to fetch subscriptions asynchronously and write the results to `subscriptions` state before requesting repaint:
  ```rust
  let state_clone = state.clone();
  let ctx_clone = cc.egui_ctx.clone();
  tokio::spawn(async move {
      let res = Self::initialize_and_fetch_async().await;
      let mut s = state_clone.lock().unwrap();
      match res {
          Ok(subs) => {
              s.subscriptions = Some(Ok(subs));
          }
          Err(e) => {
              s.subscriptions = Some(Err(e));
          }
      }
      ctx_clone.request_repaint();
  });
  ```
* **Refactor Plan**:
  * Create a static or associated helper function on `YoutubeGuiApp`:
    ```rust
    fn spawn_fetch_subscriptions(state: Arc<Mutex<AppState>>, ctx: egui::Context)
    ```
  * Call this function from both `new` and the retry handler.

---

## 2. Refactoring Tasks

1. Define `spawn_fetch_subscriptions` in `youtube-gui/src/main.rs` and simplify the duplicate async task blocks.
2. Define `init_client` in `youtube-client/src/main.rs` to deduplicate client setup across CLI subcommands.
