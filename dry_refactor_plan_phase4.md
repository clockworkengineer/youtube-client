# DRY Refactor Plan: Phase 4

This document covers the Phase 4 DRY refactoring plan.

---

## 1. Identified Duplication & Inconsistencies

### A. Duplicate and Inconsistent Credential Validation
* **Location**: `youtube-gui/src/main.rs` (in `get_client_async`) and `youtube-client/src/main.rs` (in `get_credentials`)
* **Problem**: 
  * The GUI checks config fields and filters out placeholder values like `"ENTER_YOUR_CLIENT_ID_HERE"` or `"ENTER_YOUR_CLIENT_SECRET_HERE"`.
  * The CLI does not filter out placeholders, meaning it can fail late with cryptic Google OAuth errors.
* **Refactor Plan**:
  * Add a verification method to the shared `Config` struct in `youtube-client-lib/src/lib.rs`:
    ```rust
    impl Config {
        pub fn is_valid(&self) -> bool {
            let invalid_id = |id: &str| id.is_empty() || id == "ENTER_YOUR_CLIENT_ID_HERE";
            let invalid_secret = |sec: &str| sec.is_empty() || sec == "ENTER_YOUR_CLIENT_SECRET_HERE";
            
            self.client_id.as_deref().map_or(false, |id| !invalid_id(id))
                && self.client_secret.as_deref().map_or(false, |sec| !invalid_secret(sec))
        }
    }
    ```
  * Update CLI and GUI to use `config.is_valid()` or extract clean credentials through a unified helper.

### B. Setup Instructions Duplication
* **Location**: `youtube-client/src/main.rs` (the long instruction string on Google Console configuration)
* **Problem**: This string is hardcoded in the CLI error branch, but is highly relevant to a GUI user who gets a "Google Client ID is not configured" error.
* **Refactor Plan**:
  * Move the instruction string to a shared constant or function in `youtube-client-lib`:
    ```rust
    pub const GOOGLE_SETUP_INSTRUCTIONS: &str = "...";
    ```
  * Expose this constant to both GUI and CLI.

---

## 2. Refactoring Tasks

1. Implement `Config::is_valid()` and `GOOGLE_SETUP_INSTRUCTIONS` constant in `youtube-client-lib/src/lib.rs`.
2. Refactor `get_credentials` in `youtube-client/src/main.rs` to validate loaded config values using the library helper and reuse the instructions constant.
3. Refactor `get_client_async` in `youtube-gui/src/main.rs` to validate credentials via the same logic.
