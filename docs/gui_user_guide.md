# Desktop GUI User Guide (`youtube-gui`)

`youtube-gui` is a native, 60 FPS desktop application built with `egui` and `eframe` for browsing YouTube subscriptions, filtering searches, managing playlists, inspecting SponsorBlock segments, downloading video/audio at custom resolutions and bitrates, managing an offline media library and watch history, and listening to background audio with an integrated hardware-accelerated `rodio` player.

> [!TIP]
> For a complete reference of keyboard shortcuts (`Alt+1..7`, `Ctrl+F`, `F5`, `Space`, `Esc`) and Jakob Nielsen's 10 Usability Heuristics implemented across the GUI, see the **[Usability & Keyboard Shortcuts Guide](usability_and_shortcuts.md)**.

---

## Table of Contents

1. [Starting the Application](#starting-the-application)
2. [Layout & 7 Primary Navigation Views](#layout--7-primary-navigation-views)
3. [View Walkthroughs](#view-walkthroughs)
   - [1. Login View](#1-login-view)
   - [2. Subscriptions View (`Alt+1`)](#2-subscriptions-view-alt1)
   - [3. New Videos Feed & Undo Dismissal (`Alt+2`)](#3-new-videos-feed--undo-dismissal-alt2)
   - [4. Playlists Manager (`Alt+3`)](#4-playlists-manager-alt3)
   - [5. Offline Media Library View (`Alt+4`)](#5-offline-media-library-view-alt4)
   - [6. Watch Progress History View (`Alt+5`)](#6-watch-progress-history-view-alt5)
   - [7. Search View, Filters & Recent Search Chips (`Ctrl+F`)](#7-search-view-filters--recent-search-chips-ctrlf)
   - [8. Channel Header & Uploads View](#8-channel-header--uploads-view)
   - [9. Video Details, SponsorBlock Timeline & Comments](#9-video-details-sponsorblock-timeline--comments)
   - [10. Settings View & Account Management (`Alt+6`)](#10-settings-view--account-management-alt6)
   - [11. About & Help View (`Alt+7`)](#11-about--help-view-alt7)
4. [Integrated Audio Player Dock & `🎧 Listen` Streaming](#integrated-audio-player-dock---listen-streaming)
5. [Watch Progress & Resume Playback](#watch-progress--resume-playback)
6. [Video Streaming & External Media Players](#video-streaming--external-media-players)
7. [Error Diagnosis & Actionable Recovery Banners](#error-diagnosis--actionable-recovery-banners)

---

## Starting the Application

### Via Cargo
```bash
cargo run --bin youtube-gui
# Or specify a custom log file destination:
cargo run --bin youtube-gui -- --log-file custom_client.log
```

### Via Installed Desktop Shortcut or Application Menu
If installed via `youtube-installer` or the Windows setup executable, launch **YouTube Client GUI** from your Start Menu (Windows) or Application Launcher (Linux/macOS).

---

## Layout & 7 Primary Navigation Views

The application window is organized into three primary visual zones:

1. **Top Header, Search & Navigation Bar:**
   * **Back Navigation (`⬅ Back` / `Esc`):** Traverses your view history stack (`view_stack`).
   * **7 Primary View Tabs:**
     1. **`📺 Subscriptions` (`Alt+1`)** — Subscribed channels with instant filter and unsubscribe safeguards.
     2. **`🆕 New Videos` (`Alt+2`)** — Aggregated recent uploads across subscriptions with `↩ Undo Dismiss`.
     3. **`📂 Playlists` (`Alt+3`)** — Owned and saved playlists with inline creation, editing, and item removal.
     4. **`📁 Library` (`Alt+4`)** — Offline downloaded `.mp4` videos and `.mp3` audio tracks.
     5. **`⏱ History` (`Alt+5`)** — Chronological watch progress history with resume & restart actions.
     6. **`⚙ Settings` (`Alt+6`)** — Player selection, resolution/bitrate defaults, SponsorBlock, quota gauge, and OPML/CSV/JSON backup.
     7. **`ℹ About & Help` (`Alt+7`)** — Keyboard shortcuts reference, external tool diagnostics, and version info.
   * **Global Search Bar (`Ctrl+F` or `/`):** Search YouTube from any screen with `🔍 Search`.
   * **System Status & Quota Indicators:** Displays active background task spinners (`⏳ ...`), audio buffering percentage (`🎧 Buffering audio... (N%)`), and a color-coded API Quota meter.
2. **Main Content Canvas:**
   * Smooth 60 FPS scrollable workspace rendering channel cards, video grids, filter bars, and error recovery banners.
3. **Bottom Audio Player Dock:**
   * Persistent hardware audio playback bar powered by `rodio` with Play/Pause (`Space`), Stop, `⏪ 10s` / `⏩ 10s` skip buttons, interactive timeline scrubber, elapsed/total duration (`MM:SS / MM:SS`), and volume/mute controls.

---

## View Walkthroughs

### 1. Login View

When starting without a cached OAuth token or API key, `youtube-gui` presents the Login View:
1. Click **"🔑 Sign In with Google (Browser)"** for standard desktop OAuth2 login, or **"🖥 Sign In with Device Code"** for headless/terminal verification.
2. Once approved, the refresh token is persisted with strict permissions to `tokencache.json` and your subscriptions load automatically.

---

### 2. Subscriptions View (`Alt+1`)

* **Real-Time Channel Filter:** Type in the filter box to narrow down subscribed channels by title in real time.
* **Channel Cards:** Display channel avatar thumbnail, title, description snippet, and action buttons:
  * **`🎬 View Videos`:** Opens the Channel Uploads view with full channel statistics.
  * **`➖ Unsubscribe`:** Protected by a two-step confirmation prompt (`Confirm? [Yes] [No]`) to prevent accidental unsubscribes.
* **Pagination:** Click **`⬇ Load More Subscriptions`** when `next_page_token` is available.

---

### 3. New Videos Feed & Undo Dismissal (`Alt+2`)

Aggregates recent uploads across your subscribed channels:
* **Single Video Dismissal (`✕ Dismiss`):** Removes a video card from the feed and pushes it onto the session undo stack.
* **Undo Dismiss (`↩ Undo Dismiss (N)`):** Restores the most recently dismissed video back into the feed.
* **Clear All Feeds (`🗑 Clear All Feeds`):** Prompts for confirmation before marking all currently visible feed videos as cleared in `cleared_videos.json`.
* **Zero-Quota RSS Fallback:** Automatically falls back to YouTube public Atom/RSS feeds if daily API quota is exhausted.

---

### 4. Playlists Manager (`Alt+3`)

* **Browse & Filter Playlists:** Inspect your YouTube playlists and locally saved playlists (`♥ Saved`).
* **➕ Create New Playlist:** Enter a title and optional description to create a playlist (`50 quota units`).
* **✏ Edit / 🗑 Delete Playlist:** Update playlist metadata or delete a playlist (with confirmation safeguard).
* **Playlist Videos Drill-Down (`PlaylistVideos`):** Click any playlist to view its videos, play/download items, or click **`🗑 Remove`** (with confirmation) to remove an individual video from the playlist.

---

### 5. Offline Media Library View (`Alt+4`)

Provides a dedicated manager for all media files stored in your configured `downloads_dir`:
* **Media Category Tabs:** Switch between **`All Files`**, **`🎵 Audio Only`** (`.mp3`, `.m4a`, `.wav`, `.ogg`, `.flac`), and **`🎬 Video Only`** (`.mp4`, `.mkv`, `.webm`).
* **Instant Filename Search:** Filter local files by title or video ID.
* **Storage Summary Badge:** Displays total file count and human-readable disk usage (e.g., `24 files • 1.42 GB`).
* **Per-File Actions:**
  * **`▶ Play`:** Launches the file in your configured external media player (MPV/VLC).
  * **`🎧 Listen`:** Streams downloaded audio files (`.mp3`, `.m4a`, `.wav`, `.ogg`, `.flac`) directly in the built-in bottom audio dock.
  * **`📂 Show in Folder`:** Opens the containing directory in Windows Explorer, macOS Finder, or Linux file manager.
  * **`🗑 Delete`:** Permanently deletes the file from disk after a confirmation prompt.

---

### 6. Watch Progress History View (`Alt+5`)

Displays every video with saved watch progress in `playback_positions.json`, sorted from most-recently watched to oldest:
* **Relative Timestamps & Progress Bars:** Shows when you last watched each video (`Watched 5m ago`, `Yesterday`, `3d ago`) alongside a visual completion percentage bar (`MM:SS / MM:SS (68%)`).
* **History Filter:** Search watched videos by title or video ID.
* **Actions:**
  * **`▶ Resume (MM:SS)`:** Resumes playback from your exact saved timestamp.
  * **`🔄 Restart`:** Starts playback from `00:00`.
  * **`ℹ Details`:** Opens the Video Details view for that video.
  * **`🗑 Remove` / `🗑 Clear All History`:** Remove individual entries or clear all watch history (with confirmation).

---

### 7. Search View, Filters & Recent Search Chips (`Ctrl+F`)

* **Interactive Filter Bar:** Refine YouTube searches before or after querying:
  * **Sort By:** `Relevance`, `Upload Date`, `View Count`, `Rating`, `Title`.
  * **Duration:** `Any`, `Short (< 4m)`, `Medium (4–20m)`, `Long (> 20m)`.
  * **Type:** `Video`, `Channel`, `Playlist`, `All`.
  * **Upload Date Preset:** `Any Time`, `Today`, `This Week`, `This Month`, `This Year`.
* **Recent Searches Chips (`🕒 Recent:`):** Displays up to 8 recent search queries as clickable pills for instant re-execution, plus a `✖` button to clear recent search history.
* **Quota Awareness Badge:** Shows a `100 quota units` indicator with an `ⓘ` tooltip explaining YouTube search quota costs.

---

### 8. Channel Header & Uploads View

Clicking a channel title or **"View Videos"** opens the Channel View (`channel_view.rs`):
* **Rich Channel Banner Card:** Displays the channel avatar, title, custom handle (`@handle`), **Subscriber Count**, **Total Views**, **Video Count**, description preview, and a **`➕ Subscribe`** / **`➖ Unsubscribe`** button.
* **Channel Video Grid & Pagination:** Browse uploads with watch progress indicators, one-click `▶ Play`, `🎧 Listen`, `📥 Download`, and `⬇ Load More Videos`.

---

### 9. Video Details, SponsorBlock Timeline & Comments

Clicking **`ℹ Details`** on any video card opens the comprehensive Video Details view (`details_view.rs`):

* **Engagement Metrics Bar:**
  * 👁 **Views**, 👍 **Likes**, 👎 **Dislikes (RYD)** (with like/dislike ratio bar), 💬 **Comments Count**, and ⏱ **Duration**.
* **Playback & Streaming Controls:**
  * **`▶ Play Video` / `▶ Resume (MM:SS)`:** Launches video in MPV/VLC (automatically passing SponsorBlock segments and start offset).
  * **`🎧 Listen`:** Streams audio in the background via `rodio` (extracting and caching a compatible `.mp3`/`.m4a` stream in `%TEMP%/youtube-client-audio/`). Shows real-time `🎧 Buffering audio... (N%)` progress and a **`⏹ Cancel Audio`** / **`⏹ Stop Audio`** button.
  * **`🌐 Watch in Browser`:** Opens the YouTube watch page in your default web browser.
* **Selectable Resolution & Bitrate Download Menus:**
  * **`📥 Download Video ▾`:** Choose target video resolution (`best`, `2160p`, `1440p`, `1080p`, `720p`, `480p`, `360p`) before downloading.
  * **`📥 Download Audio (MP3) ▾`:** Choose target audio bitrate (`best`, `320k`, `256k`, `192k`, `128k`) before downloading.
  * Active downloads display a live progress bar (`42.5% • 4.2 MB/s • ETA 00:12`) with a **`⏹ Cancel`** button.
* **SponsorBlock Timeline Visualization:**
  * When SponsorBlock segments exist for the video, a proportional horizontal timeline bar is rendered above the description with color-coded segments:
    * 🟩 **Sponsor** (`sponsor`)
    * 🟨 **Unpaid / Self-Promotion** (`selfpromo`)
    * 🟪 **Interaction Reminder** (`interaction`)
    * 🟦 **Intro / Intermission** (`intro`)
    * 🟦 **Endcards / Credits** (`outro`)
  * Hovering over any segment displays its category name and exact `MM:SS – MM:SS` timestamp range.
* **Collapsible Description & Tags (`📝 Video Description & Tags`):**
  * Click to expand or collapse the full description text and topic tags (`#rust`, `#programming`) without pushing comments off-screen.
* **Interactive Comments & Pagination:**
  * Post new top-level comments (`💬 Post Comment`), browse existing threads, and click **`💬 Load More Comments`** to paginate through additional comment pages.

---

### 10. Settings View & Account Management (`Alt+6`)

Configure all persistent application settings (`Ctrl+S` to save immediately):
* **Preferred Media Player:** Auto-Detect, MPV, VLC, or custom binary path.
* **Default Download Quality:** Set default **Preferred Video Resolution** (`best`, `2160p`..`360p`) and **Preferred Audio Bitrate** (`best`, `320`..`128` kbps).
* **SponsorBlock Configuration:** Enable/disable automatic SponsorBlock segment skipping and toggle individual categories (`sponsor`, `selfpromo`, `interaction`, `intro`, `outro`).
* **Downloads Directory & Cookies:** Native folder picker for `downloads_dir` and browser selector (`firefox`, `chrome`, `edge`, `brave`) for `--cookies-from-browser`.
* **API Quota Budget Gauge:** Visual progress bar (`Used: X / 10,000 units`) with manual reset safeguard.
* **Subscriptions Backup:** One-click **`📤 Export OPML`** and **`📥 Import Subscriptions (OPML/CSV/JSON)`** (see [Backup & Migration Guide](backup_and_migration.md)).
* **Account Disconnection:** Two-step confirmed **`🚪 Sign Out & Disconnect Account`**.

---

### 11. About & Help View (`Alt+7`)

* **Keyboard Shortcuts Cheat Sheet:** Quick reference table for all navigation, search, and playback shortcuts.
* **External Tool Diagnostics:** Live availability checks for `yt-dlp`, `ffmpeg`, `mpv`, and `vlc` on your system `PATH`.
* **Links & Version Info:** Direct access to log files, configuration folders, and documentation.

---

## Integrated Audio Player Dock & `🎧 Listen` Streaming

Located at the bottom of the window, the audio dock plays both local audio files and online YouTube streams:

```
[▶ Play / ⏸ Pause] [⏹ Stop] [⏪ 10s] [⏩ 10s]   03:45 / 12:30 ───●──────────   [🔊 Volume: ───●───── 80%] [🔇 Mute]
```

### How Online `🎧 Listen` Works
1. Clicking **`🎧 Listen`** on any online video triggers `PendingAction::StreamAudioDirect`.
2. If the video has already been downloaded in your `downloads_dir` or cached in `%TEMP%/youtube-client-audio/<video_id>.mp3` (or `.m4a`), playback starts **instantly (0s wait)**.
3. Otherwise, `yt-dlp` extracts a `rodio`-compatible `.mp3` stream (with automatic `.m4a` fallback if `ffmpeg` is not installed) into `%TEMP%/youtube-client-audio/` while displaying live buffering percentage (`🎧 Buffering audio... (45%)`) in both the top header and the Video Details card.
4. You can abort buffering at any time by clicking **`⏹ Cancel Audio`** in the header or Video Details view.

---

## Watch Progress & Resume Playback

* Watched timestamps are synchronized every 200ms and saved atomically to `playback_positions.json`.
* Video cards display a progress bar under the thumbnail and a `⏱ Watched to MM:SS` badge.
* Clicking **`▶ Resume (MM:SS)`** starts playback at the exact saved offset in both the internal `rodio` audio player and external players (`--start` on MPV, `--start-time` on VLC).

---

## Video Streaming & External Media Players

1. **MPV (Recommended):** Launched with `--force-window=immediate`, `--start=<seconds>`, cookie forwarding, and automatic SponsorBlock EDL / segment skipping when enabled.
2. **VLC Media Player:** Launched with `--start-time=<seconds>` and `--meta-title`.
3. **🌐 Watch in Browser:** Opens the video directly in your default web browser.

---

## Error Diagnosis & Actionable Recovery Banners

Whenever an API request, OAuth login, or `yt-dlp` command fails, `youtube-gui` renders a structured error banner at the top of the view with:
* **Plain-English Explanation:** Explains *why* the error happened (e.g., expired OAuth token, daily quota reached, missing `yt-dlp`, or YouTube cookie verification).
* **One-Click Recovery Action:** Context-sensitive buttons such as **`🔑 Re-authenticate`**, **`⚙ Open Settings`**, or **`🔄 Retry`**.
* **Technical Details & Clipboard Copy:** Expand **`🔬 Technical Details`** or click **`📋 Copy Error`** to copy the raw error log.
