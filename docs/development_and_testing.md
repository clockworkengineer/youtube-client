# Developer & Testing Guide

This guide provides setup instructions, testing conventions, mocking architectures, and CI guidelines for developers contributing to the `youtube-client` workspace.

---

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Building the Workspace](#building-the-workspace)
3. [Testing Strategy & Test Suites](#testing-strategy--test-suites)
   - [Unit Testing](#unit-testing)
   - [CLI Integration Tests](#cli-integration-tests)
   - [In-Memory Mocking with `MockYoutubeClient`](#in-memory-mocking-with-mockyoutubeclient)
4. [Static Analysis & Code Quality](#static-analysis--code-quality)
5. [Documentation Generation](#documentation-generation)

---

## Development Environment Setup

### Required Tools
* **Rust Toolchain:** Stable 1.80+ (`rustup default stable`).
* **Components:** `rustfmt` and `clippy`:
  ```bash
  rustup component add rustfmt clippy
  ```

### Optional Dependencies
* `yt-dlp`: Needed for manual end-to-end media download verification.
* `mpv` / `vlc`: For verifying external player integration.

---

## Building the Workspace

Build all crates in debug mode:
```bash
cargo build --workspace
```

Build individual crate targets:
```bash
cargo build -p youtube-client-lib
cargo build -p youtube-client
cargo build -p youtube-gui
cargo build -p youtube-installer
```

---

## Testing Strategy & Test Suites

The workspace uses a multi-layered testing strategy combining unit tests, simulated in-memory mocks, and automated CLI command parser checks.

### 1. Unit Testing

Execute all workspace tests:
```bash
cargo test --workspace
```

Run tests for the core library:
```bash
cargo test -p youtube-client-lib
```

Key unit test modules:
* `youtube-client-lib/src/retry.rs`: Validates retry logic, jitter calculation, and error classification.
* `youtube-client-lib/src/utils.rs`: Validates Windows reserved name sanitization, leading dash stripping, and atomic file replacement.
* `youtube-client-lib/src/config.rs`: Validates token cache scope checks and configuration fallback directories.

---

### 2. CLI Integration Tests

The CLI crate includes automated integration tests in [`youtube-client/tests/cli_tests.rs`](file:///c:/Projects/youtube-client/youtube-client/tests/cli_tests.rs).

Run CLI integration tests:
```bash
cargo test -p youtube-client --test cli_tests
```

#### What `cli_tests.rs` Validates:
* **`clap::Command::debug_assert()`**: Validates that all clap command definitions, argument types, value enums, and subcommands have valid syntax without conflicts.
* **All 16 Subcommands**: Tests invocation syntax, flag validation, and default value propagation for:
  `login`, `subscriptions`, `videos`, `search`, `rate`, `playlists`, `download`, `play`, `details`, `channel`, `comments`, `comment-post`, `subscribe`, `unsubscribe`, `playlist-create`, and `playlist-delete`.
* **JSON Output Handling**: Ensures `--json` flags parse correctly across all query commands.

---

### 3. In-Memory Mocking with `MockYoutubeClient`

To test API-dependent features without hitting Google API servers or consuming quota, use [`MockYoutubeClient`](file:///c:/Projects/youtube-client/youtube-client-lib/src/testing/mod.rs).

`MockYoutubeClient` implements all segregated service traits:
* `VideoService`
* `SubscriptionService`
* `PlaylistService`
* `CommentService`
* `MediaDownloader`

#### Example Usage in Tests:
```rust
use youtube_client_lib::testing::MockYoutubeClient;
use youtube_client_lib::traits::{VideoService, PlaylistService};
use youtube_client_lib::models::Rating;

#[tokio::test]
async fn test_mock_video_operations() {
    let mock = MockYoutubeClient::new();

    // Query mock videos
    let videos = mock.list_videos("channel_123", 10).await.unwrap();
    assert!(!videos.is_empty());

    // Test rating
    let rate_res = mock.rate_video("video_abc", Rating::Like).await;
    assert!(rate_res.is_ok());

    // Test playlists
    let playlists = mock.list_playlists(10).await.unwrap();
    assert!(!playlists.is_empty());
}
```

---

## Static Analysis & Code Quality

### Code Formatting
Ensure all files adhere to rustfmt style standards:
```bash
cargo fmt --all -- --check
```
Auto-format files:
```bash
cargo fmt --all
```

### Linter (Clippy)
Run clippy across all workspace members, tests, and examples:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Documentation Generation

Build complete HTML documentation with rustdoc:
```bash
cargo doc --workspace --no-deps --open
```

Ensure all doc comments compile cleanly and intra-doc links resolve without broken references.
