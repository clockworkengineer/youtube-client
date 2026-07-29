# DRY Refactor Plan: Phase 3

This document covers the Phase 3 DRY refactoring plan.

---

## 1. Identified Duplication

### A. Repetitive Thumbnail URL Extraction
* **Location**: `youtube-client-lib/src/lib.rs` (inside `list_subscriptions` and `list_videos`)
* **Problem**: Extracting the thumbnail URL is repeated:
  ```rust
  let thumbnail_url = snippet.thumbnails
      .and_then(|t| t.default)
      .and_then(|t| t.url)
      .unwrap_or_default();
  ```
* **Refactor Plan**:
  * Implement a helper function in `youtube-client-lib/src/lib.rs`:
    ```rust
    fn extract_thumbnail_url(thumbnails: Option<google_youtube3::api::ThumbnailDetails>) -> String
    ```
  * Replace the inline extractions with calls to this helper.

### B. Duplicate Path Lookups in Crate Tests
* **Location**: `youtube-client-lib/src/lib.rs` (inside `tests::test_real_connection_from_config`)
* **Problem**: Manually checks `../private_config.json` vs `../config.json` which duplicates the pattern in `load_config`.
* **Refactor Plan**:
  * Update `load_config()` to accept an optional base path (defaulting to the current working directory).
  * Update `tests::test_real_connection_from_config` to call `load_config_with_base(Path::new(".."))`.

---

## 2. Refactoring Tasks

1. Define `extract_thumbnail_url` and use it in `list_subscriptions` and `list_videos`.
2. Refactor `load_config` to support a base path parameter and update test assertions to use it.
