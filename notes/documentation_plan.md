# Comprehensive Documentation & Code Docstring Plan (`youtube-client` Workspace)

## 1. Executive Summary & Audit Findings

Following the implementation of **Phases 1–5** (`notes/feature_plan.md`), the **Usability & Heuristics Overhaul** (`notes/ui.md`), and the **Direct Audio Streaming (`🎧 Listen`) Enhancement**, a full audit of all workspace crates (`youtube-client-lib`, `youtube-client`, `youtube-gui`, `youtube-installer`) against the existing markdown documentation (`README.md`, `CHANGELOG.md`, `docs/*.md`, and crate `README.md`s) identified the following discrepancies and gaps:

1. **CLI Subcommand Count & Missing Commands (`youtube-client`):**
   - `README.md`, `youtube-client/README.md`, `docs/architecture.md`, `docs/cli_reference.md`, and `docs/development_and_testing.md` still state **17 subcommands**, whereas `youtube-client/src/cli.rs` now provides **23 subcommands**.
   - Six new subcommands (`library`, `history`, `playlist-add`, `playlist-remove`, `subscriptions-export`, `subscriptions-import`) and new flags on `search` (`--sort`, `--duration`, `--type`, `--after`), `download` (`--resolution`, `--audio-bitrate`), and `comments` (`--page-token`) are undocumented in `docs/cli_reference.md`.
2. **GUI Navigation Views, Features & Usability Heuristics (`youtube-gui`):**
   - `docs/gui_user_guide.md` still references outdated top tabs (`Downloads`, `Logs`) instead of the actual 7 navigation views (`📺 Subscriptions`, `🆕 New Videos`, `📂 Playlists`, `📁 Library`, `⏱ History`, `⚙ Settings`, `ℹ About & Help`).
   - Major GUI capabilities are missing from `docs/gui_user_guide.md` and `youtube-gui/README.md`:
     - **Offline Media Library View (`library_view.rs`)** and **Watch Progress History View (`history_view.rs`)**.
     - **Advanced Search Filters & Recent Search Chips (`search_view.rs`)**.
     - **Video Details Enhancements (`details_view.rs`)**: Resolution & bitrate selector menus, SponsorBlock color-coded timeline bar, expandable description & tags, and paginated comment loading (`💬 Load More Comments`).
     - **Channel Header Statistics (`channel_view.rs`)**: Avatar, `@handle`, subscriber/video/view counters, and confirmation dialogs.
     - **Jakob Nielsen's 10 Usability Heuristics (`views.rs`)**: Plain-language error diagnosis banners (`diagnose_error` / `render_error_banner`), contextual `ⓘ` help badges (`help_badge`), `↩ Undo Dismiss` stack in New Videos, destructive action confirmations, and full keyboard accelerator matrix (`Alt+1..7`, `Ctrl+F`/`/`, `F5`/`Ctrl+R`, `Esc`/`Alt+Left`, `Ctrl+S`, `Space`).
3. **Direct Audio Streaming (`🎧 Listen`) & SponsorBlock Architecture:**
   - `docs/architecture.md` and `docs/video_playback.md` describe `StreamAudioDirect` as piping raw stdout bytes into memory or omit it, whereas `youtube-gui/src/handlers/playback.rs` now extracts/caches compatible `.mp3` (with `.m4a` fallback) streams into `%TEMP%/youtube-client-audio/<video_id>.mp3` via `yt-dlp` with real-time buffering percentages and `CancellationToken` support (`⏹ Cancel Audio`).
   - SponsorBlock API integration (`sponsorblock.rs`) and MPV `--script-opts=ytdl_hook-try_ytdl_first=yes` / `--ytdl-raw-options= sponsorblock-mark=...` are missing from `docs/video_playback.md` and `docs/architecture.md`.
4. **Configuration Schema (`docs/configuration.md`):**
   - Missing 5 fields added to `Config` in `youtube-client-lib/src/config.rs`: `volume`, `preferred_resolution`, `preferred_audio_bitrate`, `sponsorblock_enabled`, and `sponsorblock_categories`.
5. **Test Counts & LRU Constants (`docs/development_and_testing.md`, `README.md`, `docs/architecture.md`):**
   - Documentation states `67` or `79` passing tests; the workspace now has **86 passing tests** (`35` in `youtube-client-lib`, `41` across `youtube-client` unit & integration suites, `7` in `youtube-gui`, `2` in `youtube-installer`, and `1` doc-test).
   - `docs/architecture.md` states `120` max thumbnails, whereas `AppState::MAX_THUMBNAILS` in `youtube-gui/src/types.rs` is `150`.

---

## 2. Phase 1: Add New Documentation Guides (`docs/`)

Create two focused reference guides in `docs/` and link them from `README.md` and `docs/gui_user_guide.md`:

### 2.1. Create `docs/usability_and_shortcuts.md`
* **Purpose:** Document the GUI's human-computer interaction (HCI) design, accessibility features, error recovery system, and complete keyboard shortcut reference.
* **Sections to Include:**
  1. **Jakob Nielsen's 10 Usability Heuristics Implementation Matrix:** Map each heuristic to concrete UI features in `youtube-gui`:
     - *H1 (Visibility of System Status):* Header active operation badge (`⏳ <message>`), `🎧 Buffering audio... (N%)` progress & `⏹ Cancel Audio` button, and API Quota bar color transitions.
     - *H2 (Match Between System & Real World):* Plain-English labels, relative timestamps (`Watched 5m ago` via `format_relative_time`), and human-readable byte sizes (`format_bytes`).
     - *H3 (User Control & Freedom):* Global `⬅ Back` history stack (`navigate_to`), `↩ Undo Dismiss` in New Videos (`undo_dismiss_stack`), and `CancellationToken` task aborting for downloads and audio buffering.
     - *H4 (Consistency & Standards):* Unified status color helpers (`status_Active`, `status_Success`, `status_Warning`, `status_Danger`, `status_Action`) in `youtube-gui/src/views.rs`.
     - *H5 (Error Prevention):* Explicit two-step confirmation prompts (`confirm_unsubscribe`, `confirm_clear_all_feeds`, `confirm_delete_file`, `confirm_clear_history`, `confirm_remove_playlist_item`, `confirm_logout`, `confirm_reset_quota`).
     - *H6 (Recognition Rather Than Recall):* Clickable recent search chips (`🕒 Recent:`), watched progress badges (`⏱ Watched to MM:SS`), and saved playlist heart indicators.
     - *H7 (Flexibility & Efficiency of Use):* Global keyboard shortcuts, customizable resolution/bitrate selectors, and one-click `▶ Play` / `🎧 Listen`.
     - *H8 (Aesthetic & Minimalist Design):* Collapsible **📝 Video Description & Tags** card and compact filter bars.
     - *H9 (Help Users Recognize, Diagnose & Recover from Errors):* Plain-language error classifier (`diagnose_error` & `render_error_banner`) with one-click recovery buttons (`🔑 Re-authenticate`, `⚙ Open Settings`, `🔄 Retry`, `📋 Copy Error`).
     - *H10 (Help & Documentation):* Contextual `ⓘ` hover badges (`help_badge`) and dedicated **ℹ About & Help** view.
  2. **Complete Keyboard Shortcuts Reference Table:**
     - `Alt + 1` .. `Alt + 7`: Switch directly between the 7 top-level views.
     - `Ctrl + F` or `/`: Focus the top search bar from anywhere.
     - `F5` or `Ctrl + R`: Refresh the active view (`Subscriptions`, `New Videos`, `Playlists`, `Library`, `History`).
     - `Esc` or `Alt + Left`: Navigate back in the view stack or close open confirmation modals.
     - `Space`: Toggle Play/Pause on the active background audio stream (when no text input is focused).
     - `Ctrl + S`: Save changes immediately when in the `⚙ Settings` view.

### 2.2. Create `docs/backup_and_migration.md`
* **Purpose:** Provide an end-to-end guide for exporting, importing, and migrating subscriptions and local media across platforms (YouTube / Google Takeout, FreeTube, NewPipe, RSS readers).
* **Sections to Include:**
  1. **Supported Formats (`SubscriptionFormatRegistry`):**
     - **OPML XML (`.opml`, `.xml`):** Compatible with FreeTube, NewPipe, Thunderbird, Feedly, NetNewsWire.
     - **Google Takeout CSV (`subscriptions.csv`):** Direct parser for Google Takeout exports (`Channel Id,Channel Url,Channel Title`).
     - **NewPipe JSON (`.json`):** Direct parser for Android NewPipe backup files.
  2. **CLI Export & Import Workflows:**
     - Exporting all subscriptions: `youtube-client subscriptions-export --output my_subs.opml`
     - Dry-run previewing an import file: `youtube-client subscriptions-import --file subscriptions.csv`
     - Live migration (subscribing via YouTube Data API v3): `youtube-client subscriptions-import --file freetube.opml --subscribe` (with quota impact explanation: `50 units` per channel).
  3. **GUI Import & Export Workflows (`⚙ Settings` View):**
     - Using `📤 Export OPML` and `📥 Import Subscriptions (OPML/CSV/JSON)`.
  4. **Local Media & Watch History Portability:**
     - Backing up `playback_positions.json`, `cleared_videos.json`, and the offline downloads directory (`youtube-client library --json`, `youtube-client history --json`).

---

## 3. Phase 2: Modify Existing Documentation in `docs/`

### 3.1. Update `docs/cli_reference.md`
* **File:** `c:\Projects\youtube-client\docs\cli_reference.md`
* **Modifications:**
  1. Update Table of Contents and Overview from **17 subcommands** to **23 subcommands**.
  2. Update **Subcommand 4 (`search`)**: Document new filter flags:
     - `--sort <ORDER>` (`relevance`, `date`, `rating`, `title`, `viewCount`)
     - `--duration <DURATION>` (`any`, `short`, `medium`, `long`)
     - `--type <RESOURCE_TYPE>` (`video`, `channel`, `playlist`, `all`)
     - `--after <RFC3339>` (e.g., `2025-01-01T00:00:00Z`)
  3. Update **Subcommand 9 (`comments`)**: Document `--page-token <TOKEN>` for paginating comment threads.
  4. Update **Subcommand 14 (`download`)**: Document `--resolution <RES>` (`best`, `2160p`, `1440p`, `1080p`, `720p`, `480p`, `360p`) and `--audio-bitrate <KBPS>` (`best`, `320`, `256`, `192`, `128`).
  5. Add **6 New Subcommand Reference Sections (Subcommands 18–23)**:
     - **18. `playlist-add`**: Add a video to a playlist (`--playlist-id`, `--video-id`, Cost: `50 units`).
     - **19. `playlist-remove`**: Remove an item from a playlist (`--playlist-item-id`, Cost: `50 units`).
     - **20. `subscriptions-export`**: Export all user subscriptions to OPML XML (`--output <FILE>`, Cost: `1 unit/page`).
     - **21. `subscriptions-import`**: Parse and optionally subscribe to channels from OPML, CSV, or NewPipe JSON (`--file <PATH>`, `--subscribe`, Cost: `0 units` dry-run / `50 units` per channel with `--subscribe`).
     - **22. `library`**: List, filter, search, or delete local downloaded media files (`--audio`, `--video`, `--search <QUERY>`, `--delete <FILENAME>`, `--json`, Cost: `0 units`).
     - **23. `history`**: Inspect, search, or clear local watch progress history (`--search <QUERY>`, `--remove <VIDEO_ID>`, `--clear`, `--json`, Cost: `0 units`).

### 3.2. Update `docs/gui_user_guide.md`
* **File:** `c:\Projects\youtube-client\docs\gui_user_guide.md`
* **Modifications:**
  1. **Application Layout & Navigation Bar:** Update the navigation tabs table to reflect all 7 primary views (`📺 Subscriptions`, `🆕 New Videos`, `📂 Playlists`, `📁 Library`, `⏱ History`, `⚙ Settings`, `ℹ About & Help`) plus contextual drill-down views (`ChannelVideos`, `PlaylistVideos`, `Search`, `VideoDetails`).
  2. **Search View (`search_view.rs`):** Document the interactive filter bar (Sort Order, Duration, Type, Upload Date presets `Any`/`Today`/`ThisWeek`/`ThisMonth`/`ThisYear`), clickable **Recent Searches** chips (`🕒 Recent:`) with `✖` clear button, and the API Quota impact badge.
  3. **Channel Videos View (`channel_view.rs`):** Document the rich channel header card (Channel Avatar, `@custom_url` handle, Subscriber Count, Total Views, Video Count, Description snippet, and `➕ Subscribe` / `➖ Unsubscribe` with confirmation modal).
  4. **New Videos Feed (`new_videos_view.rs`):** Document the `↩ Undo Dismiss (N)` button and confirmation dialog before `🗑 Clear All Feeds`.
  5. **Playlist Videos View (`playlist_videos_view.rs`):** Document the `🗑 Remove` button with two-step confirmation for removing videos from playlists.
  6. **New Section — Offline Media Library View (`library_view.rs`):** Document media type filter tabs (`All`, `🎵 Audio`, `🎬 Video`), real-time filename search, total storage badge, `▶ Play`, `🎧 Listen` (for `.mp3`/`.m4a`/`.wav`/`.ogg`/`.flac`), `📂 Show in Folder`, and `🗑 Delete` with confirmation.
  7. **New Section — Watch Progress History View (`history_view.rs`):** Document chronological watch list, relative timestamps (`Watched 5m ago`), progress bars, title/ID search filter, `▶ Resume` / `🔄 Restart`, `🗑 Remove`, and `🗑 Clear All History`.
  8. **Video Details View (`details_view.rs`):** Document:
     - `📥 Download Video ▾` resolution selector (`best`, `2160p`..`360p`) and `📥 Download Audio ▾` bitrate selector (`best`, `320k`..`128k`).
     - `🎧 Listen` background audio extraction & caching with real-time buffering percentage (`🎧 Buffering audio... (45%)`) and `⏹ Cancel Audio` / `⏹ Stop Audio` controls.
     - **SponsorBlock Timeline Visualization:** Color-coded segment bar (`sponsor`, `selfpromo`, `interaction`, `intro`, `outro`, `preview`, `music_offtopic`, `filler`) with hover tooltips showing exact `MM:SS - MM:SS` ranges.
     - Collapsible **📝 Video Description & Tags** panel.
     - **💬 Load More Comments** pagination button (`next_page_token`).
  9. **Error Diagnosis & Recovery Banners (`views.rs`):** Document how network, OAuth, quota, and `yt-dlp` errors are translated into actionable recovery banners.

### 3.3. Update `docs/configuration.md`
* **File:** `c:\Projects\youtube-client\docs\configuration.md`
* **Modifications:**
  1. Expand the `config.json` Schema Reference table and example JSON block with the 5 new `Config` fields in `youtube-client-lib/src/config.rs`:
     - `volume` (`Option<f32>`, default `1.0`): Persistent audio player volume (`0.0` to `1.0`).
     - `preferred_resolution` (`Option<String>`, default `"best"`): Default video download/stream resolution (`"best"`, `"2160p"`, `"1440p"`, `"1080p"`, `"720p"`, `"480p"`, `"360p"`).
     - `preferred_audio_bitrate` (`Option<String>`, default `"best"`): Default MP3 audio extraction bitrate (`"best"`, `"320"`, `"256"`, `"192"`, `"128"`).
     - `sponsorblock_enabled` (`Option<bool>`, default `true`): Enables automatic SponsorBlock segment fetching and MPV segment skipping.
     - `sponsorblock_categories` (`Option<Vec<String>>`, default `["sponsor", "selfpromo", "interaction", "intro", "outro"]`): Active SponsorBlock categories.
  2. Document the temporary audio stream cache directory (`%TEMP%/youtube-client-audio/` on Windows, `/tmp/youtube-client-audio/` on Linux/macOS) used by `🎧 Listen`.

### 3.4. Update `docs/video_playback.md`
* **File:** `c:\Projects\youtube-client\docs\video_playback.md`
* **Modifications:**
  1. **Direct Audio Streaming (`🎧 Listen`) Architecture:** Document how `PendingAction::StreamAudioDirect` extracts/caches compatible `.mp3` (with `.m4a` fallback when `ffmpeg` is absent) into `<TEMP>/youtube-client-audio/<video_id>.mp3`, reports real-time download/buffering progress to the GUI header and details view, supports instant replay from cache on subsequent listens, and aborts cleanly via `tokio_util::sync::CancellationToken` (`⏹ Cancel Audio`).
  2. **SponsorBlock Integration:** Document how `youtube-client-lib/src/sponsorblock.rs` queries `https://sponsor.ajay.app/api/skipSegments` and how `MpvPlayer` passes `--ytdl-raw-options=sponsorblock-mark=<categories>` (and how the GUI renders the color-coded segment timeline).
  3. **Resolution & Audio Bitrate Selection:** Document how `DownloadOptions` in `youtube-client-lib/src/downloader.rs` maps resolution heights (`-f bestvideo[height<=1080]+bestaudio/best[height<=1080]`) and audio bitrates (`--audio-quality 320K`) to `yt-dlp`.

### 3.5. Update `docs/architecture.md`, `docs/caching_and_quota.md`, and `docs/development_and_testing.md`
* **Files:**
  - `c:\Projects\youtube-client\docs\architecture.md`
  - `c:\Projects\youtube-client\docs\caching_and_quota.md`
  - `c:\Projects\youtube-client\docs\development_and_testing.md`
* **Modifications:**
  1. In `docs/architecture.md`:
     - Update CLI subcommand count from `17` to `23`.
     - Update `youtube-client-lib` module map to include `sponsorblock.rs` (SponsorBlock segment API client) and `DownloadOptions` in `downloader.rs`.
     - Update `youtube-gui` module map to include `views/library_view.rs`, `views/history_view.rs`, and error diagnosis/usability helpers in `views.rs`.
     - Update Section 5 (Audio & Video Playback Architecture) sequence diagram to reflect `%TEMP%/youtube-client-audio` extraction, real-time progress reporting, `CancellationToken` cancellation, and `has_active_track` state preservation in `youtube-gui/src/player.rs`.
     - Update thumbnail LRU limit from `120` to `150` (`AppState::MAX_THUMBNAILS`).
  2. In `docs/caching_and_quota.md`:
     - Add quota entries for `playlist-add` (`playlistItems.insert` = `50 units`), `playlist-remove` (`playlistItems.delete` = `50 units`), `subscriptions-export` (`1 unit` per page), `subscriptions-import` (`0 units` dry-run / `50 units` per channel with `--subscribe`), and zero-quota `library`, `history`, and SponsorBlock API calls (`0 units`).
  3. In `docs/development_and_testing.md`:
     - Update test suite count from `67` to **86 passing tests**:
       - `youtube-client-lib`: 35 unit tests (including `sponsorblock`, `importers`, `player`, `cache`, `quota`, `rss`, `retry`, `window_state`).
       - `youtube-client`: 13 binary unit tests (`formatters`, `utils`) + 28 integration tests (`cli_tests.rs` with 22 tests covering all 23 subcommands, `sanitization_tests.rs` with 5 tests, `pagination_tests.rs` with 1 test).
       - `youtube-gui`: 7 unit tests (`diagnose_error` classification, `format_relative_time`, `format_bytes`, `AppState::with_mock_backend`, `navigate_to`, `push_recent_search`).
       - `youtube-installer`: 2 unit tests.
       - Doc-tests: 1 test.

---

## 4. Phase 3: Update Root & Crate-Level `README.md`s and `CHANGELOG.md`

1. **`c:\Projects\youtube-client\README.md`**:
   - Update badges (`Tests: 86 Passed`, `CLI: 23 Subcommands`).
   - Add highlights for Offline Library, Watch History, SponsorBlock, Advanced Search Filters, Subscription Import/Export (`subscriptions-export`, `subscriptions-import`), Usability/Keyboard Shortcuts, and `🎧 Listen` background audio streaming.
   - Add links to `docs/usability_and_shortcuts.md` and `docs/backup_and_migration.md` in the Documentation Index table.
   - Update CLI Subcommands Overview table from 17 to 23 commands.
2. **`c:\Projects\youtube-client\CHANGELOG.md`**:
   - Add a new `[0.3.0] - Unreleased` (or update `[0.2.0]`) section detailing:
     - **Added:** 6 new CLI subcommands (`library`, `history`, `playlist-add`, `playlist-remove`, `subscriptions-export`, `subscriptions-import`), GUI `📁 Library` and `⏱ History` views, SponsorBlock integration + timeline bar, search filters + recent search chips, resolution/bitrate selectors, channel header statistics, comment pagination, and Jakob Nielsen usability overhaul (keyboard shortcuts, undo dismiss, error recovery banners, confirmation modals).
     - **Fixed:** `🎧 Listen` direct audio streaming reliability (`StreamAudioDirect` MP3/M4A temp extraction, buffering progress indicator, `CancellationToken` abort, and `rodio` worker state synchronization).
3. **`c:\Projects\youtube-client\youtube-client\README.md`**:
   - Update feature list from `17 Subcommands` to `23 Subcommands` and add examples for `library`, `history`, `subscriptions-export`, `subscriptions-import`, and filtered `search`/`download`.
4. **`c:\Projects\youtube-client\youtube-client-lib\README.md`**:
   - Add **SponsorBlock Integration (`sponsorblock.rs`)** and **Configurable `DownloadOptions` (Resolution & Bitrate)** to the Features list.
5. **`c:\Projects\youtube-client\youtube-gui\README.md`**:
   - Expand Features list with **Offline Media Library & Watch History Views**, **SponsorBlock Timeline Visualization**, **Resolution & Bitrate Selector Menus**, and **Jakob Nielsen Usability & Error Diagnosis**.
   - Expand Keyboard Navigation & Shortcuts section with the complete shortcut list (`Alt+1..7`, `Ctrl+F`/`/`, `F5`/`Ctrl+R`, `Esc`/`Alt+Left`, `Ctrl+S`, `Space`).

---

## 5. Phase 4: In-Code Rustdoc (`//!` and `///`) Enhancements

Ensure `cargo doc --workspace --no-deps` builds with zero warnings and comprehensive module/function documentation across all crates:

1. **`youtube-client-lib`**:
   - [`youtube-client-lib/src/lib.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/lib.rs): Update crate-level `//!` docstring architecture table to include `sponsorblock` and `DownloadOptions`.
   - [`youtube-client-lib/src/sponsorblock.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/sponsorblock.rs): Verify/expand `//!` module overview and `///` docstrings on `SponsorSegment`, `SponsorCategory`, and `fetch_sponsor_segments`.
   - [`youtube-client-lib/src/downloader.rs`](file:///c:/Projects/youtube-client/youtube-client-lib/src/downloader.rs): Ensure `DownloadOptions`, `VideoResolution`, `AudioBitrate`, and `download_video_with_options` have complete `///` doc comments and usage examples.
2. **`youtube-client` (CLI)**:
   - [`youtube-client/src/cli.rs`](file:///c:/Projects/youtube-client/youtube-client/src/cli.rs): Update top-level `//!` docstring to list all 23 subcommands.
   - [`youtube-client/src/commands/library.rs`](file:///c:/Projects/youtube-client/youtube-client/src/commands/library.rs) & [`youtube-client/src/commands/history.rs`](file:///c:/Projects/youtube-client/youtube-client/src/commands/history.rs): Ensure module `//!` headers and `///` handler docstrings document arguments, quota cost (`0 units`), and JSON output support.
3. **`youtube-gui` (Desktop GUI)**:
   - [`youtube-gui/src/player.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/player.rs): Add `//!` module-level documentation explaining the dedicated OS thread for `rodio::OutputStream`, the `has_active_track` guard during async buffering, and `PlayerCommand` handling.
   - [`youtube-gui/src/handlers/playback.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/handlers/playback.rs): Document `handle_playback_actions` and how `PendingAction::StreamAudioDirect` caches compatible audio files in `%TEMP%/youtube-client-audio`, reports buffering progress, and honors `audio_cancel_token`.
   - [`youtube-gui/src/views.rs`](file:///c:/Projects/youtube-client/youtube-gui/src/views.rs) & [`youtube-gui/src/views/`](file:///c:/Projects/youtube-client/youtube-gui/src/views/mod.rs): Ensure `diagnose_error`, `render_error_banner`, `help_badge`, `format_relative_time`, `format_bytes`, `library_view.rs`, and `history_view.rs` have clear `//!` and `///` rustdoc comments referencing the usability heuristics they implement.

---

## 6. Verification Checklist

After executing Phases 1–4:
1. Run `cargo test --workspace` to verify all unit, integration, and doc-tests pass (`86 passed`).
2. Run `cargo clippy --workspace --all-targets -- -D warnings` to ensure zero warnings.
3. Run `cargo doc --workspace --no-deps` to verify all rustdoc comments and intra-doc links compile cleanly without warnings.

