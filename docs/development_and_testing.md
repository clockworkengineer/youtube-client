# Developer & Testing Guide (v0.2.0)

This guide provides setup instructions, testing conventions, mocking architectures, static analysis requirements, and CI guidelines for developers contributing to the `youtube-client` workspace.

---

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Building the Workspace](#building-the-workspace)
3. [Testing Strategy & Test Suites](#testing-strategy--test-suites)
   - [Overview of the 67 Test Suites](#overview-of-the-67-test-suites)
   - [Core Unit Testing](#core-unit-testing)
   - [CLI Integration & Parser Testing](#cli-integration--parser-testing)
   - [In-Memory Mocking & `YoutubeBackend` Injection](#in-memory-mocking--youtubebackend-injection)
   - [OutputFormatter Testing](#outputformatter-testing)
4. [Static Analysis & Code Quality](#static-analysis--code-quality)
5. [SOLID Contribution Standards](#solid-contribution-standards)
6. [Documentation Generation](#documentation-generation)

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

The workspace employs an exhaustive testing hierarchy combining isolated unit tests, in-memory simulated mocks, end-to-end command parser validations, and format serialization tests.

### Overview of the 67 Test Suites

The test matrix consists of **67 passing tests** with 0 network or credentials dependencies:
* **`youtube-client-lib` (31 unit tests):** Quota tracker, daily UTC rollover, TTL cache insertion/expiration/prune, PlayOptions builder, PlayerRegistry auto-discovery, RSS Atom XML parser, retry logic, sanitization, token cache scope checks, and window state serialization.
* **`youtube-client` (4 unit tests):** `JsonFormatter`, `CsvFormatter`, `TableFormatter`, and paginated page token layout.
* **`cli_tests.rs` (15 integration tests):** Clap debug assert, all 17 subcommand parser validations, and mock backend command dispatch.
* **`utils.rs` (8 tests):** Video ID extraction, filename sanitization, RYD API deserialization, downloads directory scanner, playback positions persistence, and string table formatting.
* **`sanitization_tests.rs` (5 tests):** Windows reserved names (`CON`, `PRN`, `NUL`), length truncation, leading dashes, Unicode, and edge cases.
* **`pagination_tests.rs` (1 test):** Page lifecycle and resumption token chaining.
* **`youtube-installer` (2 tests):** Linux path export line generation and default directory resolution.
* **Doc-tests (1 test):** Executable quickstart verification in `youtube-client-lib/src/lib.rs`.

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
use youtube_client::formatters::{select_formatter, JsonFormatter, CsvFormatter, TableFormatter};
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
    };

    let formatter = TableFormatter;
    let table = formatter.format_list(&[video]);
    assert!(table.contains("Index"));
    assert!(table.contains("Title"));
    assert!(table.contains("Video ID"));
}
```

---

## Static Analysis & Code Quality

### Code Formatting
Ensure all files adhere to rustfmt style standards:
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

## SOLID Contribution Standards

When adding new features or refactoring existing modules:
1. **Single Responsibility (SRP):** Keep domain handlers focused. Place new UI actions into [`youtube-gui/src/handlers/`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/), not in `main.rs`.
2. **Open/Closed (OCP):**
   - Add new media players by implementing [`MediaPlayer`](file:///c:/Projects/youtube-client/youtube-client-lib/src/player.rs).
   - Add new subscription file formats by implementing [`SubscriptionFormat`](file:///c:/Projects/youtube-client/youtube-client-lib/src/importers.rs).
   - Add new terminal formats by implementing [`OutputFormatter<T>`](file:///c:/Projects/youtube-client/youtube-client/src/formatters.rs).
3. **Liskov Substitution (LSP):** Ensure any new operations added to `YoutubeApiService` are implemented identically by both `YoutubeClient` and `MockYoutubeClient`.
4. **Interface Segregation (ISP):** Avoid loose, sprawling function parameter lists. Group view rendering data into cohesive context objects (like `VideoDetailsContext`).
5. **Dependency Inversion (DIP):** Consumer crates must depend on `YoutubeBackend` or service traits, never on concrete Google API network structs.

---

## Documentation Generation

Build complete HTML documentation with rustdoc:
```bash
cargo doc --workspace --no-deps --open
```

Ensure all doc comments compile cleanly and intra-doc links resolve without warnings.
