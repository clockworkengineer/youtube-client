# Developer & Testing Guide (v0.3.0)

This guide provides setup instructions, testing conventions, mocking architectures, static analysis requirements, and CI guidelines for developers contributing to the `youtube-client` workspace.

---

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Building the Workspace](#building-the-workspace)
3. [Testing Strategy & Test Suites](#testing-strategy--test-suites)
   - [Overview of the 86 Passing Tests](#overview-of-the-86-passing-tests)
   - [In-Memory Mocking & `YoutubeBackend` Injection](#in-memory-mocking--youtubebackend-injection)
   - [OutputFormatter Testing](#outputformatter-testing)
   - [GUI Usability & State Unit Testing](#gui-usability--state-unit-testing)
4. [Static Analysis & Code Quality](#static-analysis--code-quality)
5. [SOLID & Usability Contribution Standards](#solid--usability-contribution-standards)
6. [Documentation Generation](#documentation-generation)

---

## Development Environment Setup

### Required Tools
* **Rust Toolchain:** Stable 1.80+ (`rustup default stable`).
* **Components:** `rustfmt` and `clippy`:
  ```bash
  rustup component add rustfmt clippy
  ```

### Optional Runtime Dependencies
* `yt-dlp`: Needed for manual end-to-end media download and `🎧 Listen` audio extraction verification.
* `ffmpeg`: Used by `yt-dlp` for MP3 transcoding and high-resolution video/audio container muxing (when absent, `🎧 Listen` automatically falls back to native `.m4a`).
* `mpv` / `vlc`: For verifying external video player integration.

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

The workspace employs an exhaustive testing hierarchy combining isolated unit tests, in-memory simulated mocks, end-to-end CLI command parser validations, GUI state/usability tests, and format serialization tests.

### Overview of the 86 Passing Tests

The test matrix consists of **86 passing tests** with **0 network or live credential dependencies**:

* **`youtube-client-lib` (35 unit tests):**
  - `QuotaTracker` cost calculation, daily UTC rollover, and atomic disk persistence.
  - `TtlCache` & `MetadataCache` insertion, expiration, pruning, and disk reload.
  - `PlayOptions` fluent builder and `PlayerRegistry` auto-discovery.
  - Multi-format `SubscriptionFormatRegistry` (OPML XML, Google Takeout CSV, NewPipe JSON) detection, parsing, and serialization.
  - `sponsorblock` segment parsing and MPV `edl://` URL construction.
  - Public YouTube Atom/RSS XML parser (`parse_youtube_rss_xml`).
  - Exponential backoff retry logic (`retry_api_call`), filename sanitization, token cache scope checks, and window state serialization.
* **`youtube-client` Binary Unit Tests (13 tests):**
  - `JsonFormatter`, `CsvFormatter`, `TableFormatter`, and paginated `Page<T>` token layout.
  - Video ID extraction, filename sanitization, Return YouTube Dislike (RYD) deserialization, downloads directory scanner (`scan_downloads_dir`), and `playback_positions.json` persistence.
* **`youtube-client` Integration Tests (`tests/` — 28 tests):**
  - `cli_tests.rs` (22 tests): Clap `debug_assert`, parser validation across all **23 CLI subcommands** (`search` filters, `download` resolution/bitrate, `export-subscriptions`, `import-subscriptions`, `playlist-remove-video`, `playlist-update`, `library`, `history`), and mock backend command execution.
  - `sanitization_tests.rs` (5 tests): Windows reserved names (`CON`, `PRN`, `NUL`), length truncation, leading dashes, Unicode, and path traversal prevention.
  - `pagination_tests.rs` (1 test): Multi-page lifecycle and resumption token chaining.
* **`youtube-gui` Unit Tests (7 tests):**
  - `diagnose_error` classification across OAuth/401, Quota/403, missing `yt-dlp`/`ffmpeg`, cookie verification, and network timeout categories.
  - `format_relative_time` (`Just now`, `5m ago`, `2h ago`, `3d ago`) and `format_bytes` (`B`, `KB`, `MB`, `GB`) formatting.
  - `AppState::with_mock_backend` initialization, `navigate_to` view history stack management, and `push_recent_search` deduplication & capacity capping.
* **`youtube-installer` Unit Tests (2 tests):**
  - Linux shell `PATH` export line generation and platform default installation directory resolution.
* **Doc-tests (1 test):**
  - Executable quickstart verification in `youtube-client-lib/src/lib.rs`.

Run all tests across the workspace:
```bash
cargo test --workspace
```

---

### In-Memory Mocking & `YoutubeBackend` Injection

`youtube-client-lib` provides [`MockYoutubeClient`](file:///c:/Projects/youtube-client/youtube-client-lib/src/testing/mod.rs), an in-memory test double implementing `YoutubeApiService`.

#### Testing CLI Commands with Mock Backend
You can test CLI commands end-to-end without network access using `CliContext::with_mock`:

```rust
use youtube_client::commands::{CliContext, execute_subscribe, execute_rate};
use youtube_client_lib::MockYoutubeClient;
use youtube_client_lib::models::Rating;

#[tokio::test]
async fn test_cli_mock_execution() {
    let mock = MockYoutubeClient::new();
    let ctx = CliContext::with_mock(mock.clone());

    // Execute subscribe subcommand
    execute_subscribe(&ctx, "UC_rust_lang".to_string()).await.unwrap();
    assert_eq!(mock.subscriptions.lock().unwrap().len(), 1);

    // Execute rate subcommand
    execute_rate(&ctx, "video_999".to_string(), Rating::Like).await.unwrap();
    assert_eq!(
        mock.video_ratings.lock().unwrap().get("video_999"),
        Some(&"like".to_string())
    );
}
```

#### Testing GUI State with Mock Backend
In `youtube-gui`, inject the mock backend directly into `AppState`:
```rust
let mock = MockYoutubeClient::new();
let state = AppState::with_mock_backend(mock);
assert!(state.backend.is_mock());
```

---

### OutputFormatter Testing

Formatters implement the [`OutputFormatter<T>`](file:///c:/Projects/youtube-client/youtube-client/src/formatters.rs) trait, allowing pure string assertion tests without stdout capture:

```rust
use youtube_client::formatters::{OutputFormatter, TableFormatter};
use youtube_client_lib::models::Video;

#[test]
fn test_video_table_formatter() {
    let video = Video {
        id: "vid_1".to_string(),
        title: "Test".to_string(),
        description: "Desc".to_string(),
        published_at: "2026-01-01".to_string(),
        thumbnail_url: "http://thumb.jpg".to_string(),
        channel_title: "Rust".to_string(),
        channel_id: Some("UC_rust".to_string()),
        playlist_item_id: None,
    };

    let formatter = TableFormatter;
    let table = formatter.format_list(&[video]);
    assert!(table.contains("Index"));
    assert!(table.contains("Title"));
    assert!(table.contains("Video ID"));
}
```

---

### GUI Usability & State Unit Testing

Usability helpers in [`youtube-gui/src/views.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/views.rs) and state transitions in [`youtube-gui/src/types.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/types.rs) are pure functions testable without launching an OpenGL/winit window:

```rust
use youtube_gui::views::{diagnose_error, format_bytes};

#[test]
fn test_error_diagnosis_and_bytes() {
    let diag = diagnose_error("HTTP 403 Forbidden: quotaExceeded");
    assert!(diag.summary.to_lowercase().contains("quota"));
    assert_eq!(format_bytes(1_572_864), "1.5 MB");
}
```

---

## Static Analysis & Code Quality

### Code Formatting
Ensure all files adhere to `rustfmt` style standards:
```bash
cargo fmt --all -- --check
```

Auto-format all code:
```bash
cargo fmt --all
```

### Linter (Clippy)
All PRs and workspace builds enforce **zero warnings** with `-D warnings`:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

---

## SOLID & Usability Contribution Standards

When adding new features or refactoring existing modules:
1. **Single Responsibility (SRP):** Keep domain handlers focused. Place new UI actions into [`youtube-gui/src/handlers/`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/) and new views into [`youtube-gui/src/views/`](file:///c:/Projects/youtube-client/youtube-gui/src/views/), not in `main.rs`.
2. **Open/Closed (OCP):**
   - Add new media players by implementing [`MediaPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs).
   - Add new subscription file formats by implementing [`SubscriptionFormat`](file:///c:/Projects/youtube-client/youtube-client-lib/src/importers.rs).
   - Add new terminal formats by implementing [`OutputFormatter<T>`](file:///c:/Projects/youtube-client/youtube-client/src/formatters.rs).
3. **Liskov Substitution (LSP):** Ensure any new operations added to `YoutubeApiService` are implemented identically by both `YoutubeClient` and `MockYoutubeClient`.
4. **Interface Segregation (ISP):** Group view rendering data into cohesive context objects (like `VideoDetailsContext`).
5. **Dependency Inversion (DIP):** Consumer crates must depend on `YoutubeBackend` or service traits, never on concrete Google API network structs.
6. **Jakob Nielsen's 10 Usability Heuristics:** Any new GUI destructive action must include a two-step confirmation flag in `AppState`, long-running tasks must show progress and support cancellation where applicable, and errors must route through `render_error_banner` (see [`docs/USABILITY_AND_SHORTCUTS.md`](USABILITY_AND_SHORTCUTS.md)).

---

## Documentation Generation

Build complete HTML documentation with `rustdoc`:
```bash
cargo doc --workspace --no-deps --open
```

Ensure all doc comments compile cleanly and intra-doc links resolve without warnings.
