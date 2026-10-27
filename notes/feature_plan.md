# YouTube Client Suite: Feature Gap Analysis & Implementation Roadmap

> **Target Version:** `v0.4.0`  
> **Status:** Concrete Implementation Plan  
> **Workspaces Affected:** `youtube-client-lib`, `youtube-client`, `youtube-gui`

---

## Executive Summary & Feature Gap Matrix

An audit of the `youtube-client` workspace reveals substantial core engine capabilities in `youtube-client-lib` (such as SponsorBlock EDL generation, MPV IPC socket control, direct Rodio streaming, and OPML/CSV subscription importers) that are not yet connected to the user-facing CLI or GUI applications. Furthermore, critical UX features standard in modern YouTube clients (offline library browsing, watch history management, rich search filtering, video resolution/quality selection, and playlist item removal) are currently missing.

| Feature Area | Current State | Missing Capability | Target Subsystem | Priority |
| :--- | :--- | :--- | :--- | :---: |
| **Offline Library** | Downloads save to disk; feed cards detect local files | No dedicated "Library" / "Downloads" view to browse offline media | `youtube-gui`, `youtube-client` | **P0** |
| **Watch History** | `playback_positions.json` tracks timestamps | No "History" view or CLI command to browse past videos & resume | `youtube-gui`, `youtube-client` | **P0** |
| **Search Filtering** | Raw query string with hardcoded `type=video` | No sort by upload date/views/rating; no duration or type filters | `youtube-client-lib`, `youtube-client`, `youtube-gui` | **P0** |
| **Direct Audio Streaming** | Only plays local `.mp3` files in bottom dock | No in-app audio streaming from URL without prior download (`stream_audio_rodio`) | `youtube-gui`, `youtube-client` | **P1** |
| **SponsorBlock Integration** | `sponsorblock.rs` generates MPV EDL | No UI toggle, segment indicators in scrubber, or CLI `--sponsorblock` flag | `youtube-gui`, `youtube-client` | **P1** |
| **Download Quality Picker** | Fixed default download format (`mp4` / `mp3`) | No resolution selector (1080p, 720p, 480p) or audio bitrate choice | `youtube-client-lib`, `youtube-gui` | **P1** |
| **Playlist Item Removal** | `add_to_playlist` & `delete_playlist` only | Cannot remove a video from an owned playlist; item IDs are discarded | `youtube-client-lib`, `youtube-gui`, `youtube-client` | **P1** |
| **CLI Subscriptions Import/Export** | Available in GUI Settings only | No CLI `import` or `export` subcommands for OPML / CSV backups | `youtube-client` | **P2** |
| **Channel Statistics Header** | Channel view only lists recent video uploads | Channel header banner, subscriber count, total views, avatar full-size | `youtube-gui` | **P2** |
| **Comments Pagination** | Single first page of 20 comments | "Load more comments" button with pagination token handling | `youtube-gui` | **P2** |
| **MPV IPC Realtime Sync** | `mpv_ipc.rs` client implemented | External MPV playback progress is not synced back to GUI in real-time | `youtube-gui` | **P3** |

---

## 1. Feature Architecture & Specification

### 1.1 Offline Library & Downloads Management View
- **Problem:** When offline or without internet connectivity, users cannot launch or browse their previously downloaded videos and audio files. Although `youtube-client-lib::utils::scan_downloads_dir` exists, there is no GUI navigation item or CLI subcommand to browse downloaded media.
- **Library & GUI Design:**
  - Add `View::Library` to `youtube-gui/src/types.rs`.
  - Add `LibraryView` component in `youtube-gui/src/views/library_view.rs`:
    - Filter tabs: `All`, `Audio (MP3)`, `Video (MP4)`.
    - Search filter within downloaded media titles.
    - Information cards displaying: thumbnail preview (if extracted/cached), title, file size (MB/GB), duration, and local file path.
    - Direct Action buttons: `▶ Play Local`, `📂 Open Folder`, `🗑 Delete File` (with confirmation).
  - Add `youtube-client library` CLI subcommand:
    - Lists all downloaded files formatted as Table, JSON, or CSV with file sizes and paths.

### 1.2 Watch History View & Resume Management
- **Problem:** `playback_positions.json` silently captures `position_secs`, `duration_secs`, and `updated_at` for every played file. However, users have no way to view their watch history, resume older videos, or prune history.
- **Design:**
  - Add `View::History` in `youtube-gui/src/types.rs` and `youtube-gui/src/views/history_view.rs`.
  - Read `playback_positions.json` and sort by `updated_at` descending.
  - Display progress bar under each entry (`Watched 12:45 / 24:10 - 52%`).
  - Actions:
    - `▶ Resume`: Launches at recorded start offset.
    - `↺ Restart`: Launches at 0:00.
    - `❌ Remove`: Evicts single entry from history map.
    - `Clear All History`: Purges `playback_positions.json` with confirmation.
  - CLI: Add `youtube-client history [--clear] [--limit <N>]`.

### 1.3 Search Filtering & Ordering Engine
- **Problem:** The YouTube Data API v3 provides comprehensive search filtering (`order`, `videoDuration`, `publishedAfter`, `type`), but `youtube-client-lib` hardcodes `type="video"` and exposes only `query: &str, max_results: u32`.
- **API Extension in `youtube-client-lib`:**
  ```rust
  #[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct SearchFilter {
      pub order: Option<SearchOrder>,          // Relevance, Date, ViewCount, Rating
      pub duration: Option<DurationFilter>,    // Any, Short (<4m), Medium (4-20m), Long (>20m)
      pub content_type: Option<SearchType>,   // Video, Channel, Playlist
      pub published_after: Option<String>,    // Today, ThisWeek, ThisMonth, ThisYear
  }
  ```
- **CLI Flags:**
  ```powershell
  youtube-client search --query "Rust async" --order view-count --duration medium --type video
  ```
- **GUI Filter Bar:**
  - Add an expandable filter drawer in `search_view.rs` with egui `ComboBox` dropdowns for Sort, Duration, Content Type, and Upload Date.

### 1.4 Direct In-App Audio Streaming (URL to Rodio)
- **Problem:** In `youtube-gui`, clicking "Play Audio" on feed cards is only enabled if the video was already downloaded to an `.mp3` file. `youtube-client-lib` already contains `stream_audio_rodio`, which pipes `yt-dlp -f bestaudio -o -` directly into `rodio` decoding.
- **Design:**
  - Update `PendingAction::PlayLocal` to distinguish local files from `PendingAction::StreamAudioDirect { video_id, title }`.
  - In `youtube-gui/src/player.rs`: Handle `PlayerCommand::StreamAudioUrl { video_id, title, start_secs }`.
  - Launch `youtube_client_lib::audio::stream_audio_rodio` in the background audio thread, streaming audio packets directly into the Rodio sink.
  - Users can click `🎧 Listen` on ANY video card in Subscriptions, New Videos, Search, or Channel views to instantly hear the audio without waiting for a full file download.

### 1.5 SponsorBlock & MPV EDL Integration
- **Problem:** `youtube-client-lib/src/sponsorblock.rs` queries the SponsorBlock API and constructs MPV `edl://` virtual playlist streams. However, neither the GUI nor the CLI exposes this feature.
- **Design:**
  - Add `sponsorblock_enabled: bool` and `sponsorblock_categories: Vec<String>` to `Config` in `youtube-client-lib`.
  - Add SponsorBlock settings toggles in `SettingsView` (Sponsors, Intro/Outro, Self-promotion, Interaction).
  - When streaming via MPV:
    - If SponsorBlock is enabled, fetch skip segments and launch MPV with the `edl://` URI, or pass `--script-opts=sponsorblock` commands.
  - In `details_view.rs`: Display detected sponsor intervals as colored segments along the timeline and in an informational badge.
  - CLI: Add `--sponsorblock` flag to `play` and `download` commands.

### 1.6 Download Quality & Format Selector
- **Problem:** Currently, clicking Download triggers `spawn_download` with either audio (`mp3`) or video (`mp4`), with no user control over resolution (1080p vs 720p vs 4K) or audio quality.
- **Design:**
  - Define `DownloadQuality` enum:
    ```rust
    pub enum VideoResolution { Res360p, Res480p, Res720p, Res1080p, Res1440p, Res2160p, Best }
    pub enum AudioBitrate { Kbps128, Kbps192, Kbps256, Kbps320, Best }
    ```
  - In `DownloadOptions`, map the selected resolution to `yt-dlp` format specifiers:
    - e.g., `bestvideo[height<=1080]+bestaudio/best[height<=1080]`
  - In `youtube-gui`:
    - Add a download options modal or popup when clicking `Download`: allows picking format and resolution before starting.
    - Add default preferred resolution in `SettingsView`.

### 1.7 Playlist Item Deletion & Enhanced Playlist Editing
- **Problem:** Users can create playlists and add videos to playlists, but cannot remove videos from playlists. When `list_playlist_videos` is called, `PlaylistItem.id` (the deletion key) is discarded.
- **Design:**
  - Extend `Video` struct with `pub playlist_item_id: Option<String>`.
  - Add `remove_from_playlist(playlist_item_id: &str)` to `PlaylistService` trait and `YoutubeBackend`.
  - In `playlist_videos_view.rs`:
    - Render a `🗑 Remove` button on each video card if the playlist belongs to the authenticated user.
    - Dispatch `PendingAction::RemoveFromPlaylist { playlist_id, playlist_item_id }`.
  - Add CLI command: `youtube-client playlist-remove-video --playlist-id <PID> --playlist-item-id <ITEM_ID>`.

### 1.8 Subscription Import / Export via CLI
- **Problem:** Subscription import and export (OPML, Google Takeout CSV, NewPipe JSON) is only accessible in the GUI Settings view. Users working in terminal/headless environments cannot backup or restore subscriptions.
- **Design:**
  - Add subcommands in `youtube-client`:
    - `youtube-client export-subscriptions --format <opml|csv|json> --output <FILE>`
    - `youtube-client import-subscriptions --format <auto|opml|csv|json> --file <FILE>`

---

## 2. Phased Implementation Roadmap

### Phase 1: Core Domain Extensions & Playlist Mutations
- [x] **1.1:** Add `playlist_item_id: Option<String>` to `Video` model in `youtube-client-lib`.
- [x] **1.2:** Update `list_playlist_videos_page` in `client.rs` to populate `playlist_item_id`.
- [x] **1.3:** Add `remove_from_playlist(&self, playlist_item_id: &str) -> Result<()>` to `PlaylistService` trait, `YoutubeBackend`, and `MockYoutubeClient`.
- [x] **1.4:** Add `update_playlist(&self, playlist_id: &str, title: &str, description: Option<&str>)` to `PlaylistService`.
- [x] **1.5:** Add `SearchFilter`, `SearchOrder`, `DurationFilter`, and `SearchType` enums and update `search_videos_page` in `youtube-client-lib`.

### Phase 2: Offline Library & Watch History Subsystems
- [x] **2.1:** Implement `LibraryView` in `youtube-gui/src/views/library_view.rs` with audio/video filtering and file deletion.
- [x] **2.2:** Add `View::Library` and top navigation button in `youtube-gui`.
- [x] **2.3:** Implement `HistoryView` in `youtube-gui/src/views/history_view.rs` displaying `playback_positions.json` with progress bars and resume buttons.
- [x] **2.4:** Add `View::History` and top navigation button in `youtube-gui`.
- [x] **2.5:** Add CLI commands `youtube-client library` and `youtube-client history [--clear]`.

### Phase 3: Direct Streaming & Quality Control
- [x] **3.1:** Integrate `stream_audio_rodio` into GUI playback handler (`PendingAction::StreamAudioDirect`), enabling in-app audio listening on any video without full download.
- [x] **3.2:** Add `DownloadQuality` resolution and bitrate specifier to `DownloadOptions` in `youtube-client-lib`.
- [x] **3.3:** Add quality picker dialog in `youtube-gui/src/views/details_view.rs` and default quality setting in `settings_view.rs`.
- [x] **3.4:** Add `--resolution` and `--audio-quality` flags to `youtube-client download`.

### Phase 4: SponsorBlock & Search UX Enhancements
- [ ] **4.1:** Add SponsorBlock category toggles to `Config` and `SettingsView`.
- [ ] **4.2:** Integrate `build_mpv_edl` into media player launcher when SponsorBlock is active.
- [ ] **4.3:** Display detected sponsor intervals as visual markers in `details_view.rs`.
- [ ] **4.4:** Add search filter toolbar (Sort, Duration, Type, Date) in `youtube-gui/src/views/search_view.rs`.
- [ ] **4.5:** Add `--order`, `--duration`, and `--type` flags to `youtube-client search`.

### Phase 5: CLI Backup Interoperability & Channel Header
- [ ] **5.1:** Add `export-subscriptions` and `import-subscriptions` commands to `youtube-client`.
- [ ] **5.2:** Fetch and render channel header statistics (`subscriber_count`, `video_count`, `view_count`) in `youtube-gui/src/views/channel_view.rs`.
- [ ] **5.3:** Add "Load More Comments" pagination button in `youtube-gui/src/views/details_view.rs`.
- [ ] **5.4:** Verify all existing and new unit/integration tests pass (`cargo test --workspace`).

---

## 3. Verification & Acceptance Criteria

1. **Library & History Acceptance:**
   - Turning off network adapter allows browsing downloaded files in `Library` view and playing them locally.
   - Playing any video records progress in `History` view; clicking `Resume` resumes within 1 second of last watched position.
2. **Search Filters Acceptance:**
   - Searching with `--duration short` returns only videos under 4 minutes.
   - Searching with `--type playlist` returns only playlists.
3. **Audio Streaming Acceptance:**
   - Clicking `🎧 Stream Audio` on an undownloaded video begins audio playback through Rodio within 2–3 seconds.
4. **SponsorBlock Acceptance:**
   - Launching an MPV stream for a sponsored video skips sponsor segments automatically according to user-selected categories.
5. **Regression Verification:**
   - All 80+ existing workspace tests continue to pass without error.
