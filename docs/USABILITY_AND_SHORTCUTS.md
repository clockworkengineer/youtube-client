# Usability Heuristics, Error Recovery & Keyboard Shortcuts Guide (v0.3.0)

This guide documents the human-computer interaction (HCI) architecture of `youtube-gui`, mapping its design directly to **Jakob Nielsen's 10 Usability Heuristics**, detailing the automated error diagnosis and recovery system, and providing a complete keyboard shortcut reference.

---

## Table of Contents

1. [Jakob Nielsen's 10 Usability Heuristics Implementation](#jakob-nielsens-10-usability-heuristics-implementation)
2. [Global Keyboard Shortcuts Reference](#global-keyboard-shortcuts-reference)
3. [Automated Error Diagnosis & Recovery System](#automated-error-diagnosis--recovery-system)
4. [Destructive Action Confirmation Safeguards](#destructive-action-confirmation-safeguards)
5. [Contextual Help & Visual Status Indicators](#contextual-help--visual-status-indicators)

---

## Jakob Nielsen's 10 Usability Heuristics Implementation

Every view in `youtube-gui` (`youtube-gui/src/views/`) is engineered around Jakob Nielsen's 10 foundational usability principles:

| # | Usability Heuristic | Concrete Implementation in `youtube-gui` | Source Locations |
| :---: | :--- | :--- | :--- |
| **1** | **Visibility of System Status** | • Top bar displays an active operation spinner (`⏳ <task>`) with a `✖ Cancel` button for long-running tasks.<br>• Background audio streaming (`🎧 Listen`) displays real-time extraction percentage (`🎧 Buffering audio... (45%)`) and an immediate `⏹ Cancel Audio` button.<br>• Daily API Quota progress bar (`Quota: X / 10,000 units`) transitions from green (`<50%`) to amber (`50–80%`) to red (`>80%`). | [`main.rs`](../youtube-gui/src/main.rs), [`details_view.rs`](../youtube-gui/src/views/details_view.rs), [`playback.rs`](../youtube-gui/src/handlers/playback.rs) |
| **2** | **Match Between System and the Real World** | • Uses clear, non-technical labels (`📺 Subscriptions`, `🆕 New Videos`, `📁 Library`, `⏱ History`).<br>• Converts raw Unix timestamps into natural relative times (`Watched 5m ago`, `2d ago` via `format_relative_time`).<br>• Formats raw file byte counts into human-readable units (`14.2 MB`, `1.08 GB` via `format_bytes`). | [`views.rs`](../youtube-gui/src/views.rs), [`history_view.rs`](../youtube-gui/src/views/history_view.rs), [`library_view.rs`](../youtube-gui/src/views/library_view.rs) |
| **3** | **User Control and Freedom** | • Persistent `⬅ Back` button and `Esc` / `Alt+Left` shortcuts traverse the navigation stack (`view_stack`).<br>• Dismissing videos in `🆕 New Videos` pushes entries onto an undo stack, exposing `↩ Undo Dismiss (N)` in the toolbar.<br>• Active media downloads and audio stream buffering can be aborted mid-flight via `tokio_util::sync::CancellationToken`. | [`navigation.rs`](../youtube-gui/src/handlers/navigation.rs), [`new_videos_view.rs`](../youtube-gui/src/views/new_videos_view.rs), [`download.rs`](../youtube-gui/src/handlers/download.rs) |
| **4** | **Consistency and Standards** | • Standardized semantic color palette across all views (`COLOR_ACTIVE`, `COLOR_SUCCESS`, `COLOR_WARNING`, `COLOR_DANGER`, `COLOR_ACTION`).<br>• Uniform action button hierarchy across feed cards, search results, playlists, and video details (`▶ Play`, `🎧 Listen`, `📥 Download`, `ℹ Details`). | [`views.rs`](../youtube-gui/src/views.rs) |
| **5** | **Error Prevention** | • All destructive actions require explicit two-step confirmation before execution (unsubscribing, clearing all feeds, deleting offline files, clearing watch history, removing playlist items, logging out, resetting quota).<br>• Buttons that cannot be executed (e.g., empty search query, active download in progress) are disabled with explanatory hover tooltips. | [`types.rs`](../youtube-gui/src/types.rs), [`settings_view.rs`](../youtube-gui/src/views/settings_view.rs), [`library_view.rs`](../youtube-gui/src/views/library_view.rs) |
| **6** | **Recognition Rather Than Recall** | • Search view displays clickable **Recent Searches** chips (`🕒 Recent:`) so users can re-run queries with a single click.<br>• Video cards display `⏱ Watched to MM:SS` progress badges and swap `▶ Play` for `▶ Resume (MM:SS)`.<br>• Saved playlists show `♥ Saved` indicators without requiring navigation into Settings. | [`search_view.rs`](../youtube-gui/src/views/search_view.rs), [`playlists_view.rs`](../youtube-gui/src/views/playlists_view.rs) |
| **7** | **Flexibility and Efficiency of Use** | • Full suite of global keyboard shortcuts (`Alt+1..7`, `Ctrl+F`/`/`, `F5`/`Ctrl+R`, `Space`, `Esc`, `Ctrl+S`).<br>• Configurable default video resolution (`best`, `2160p`..`360p`) and audio bitrate (`best`, `320k`..`128k`) in Settings, plus per-download override menus in Video Details. | [`main.rs`](../youtube-gui/src/main.rs), [`details_view.rs`](../youtube-gui/src/views/details_view.rs) |
| **8** | **Aesthetic and Minimalist Design** | • Long video descriptions and tag lists in `VideoDetails` are tucked inside a collapsible `📝 Video Description & Tags` card so comments and playback controls remain immediately visible above the fold.<br>• SponsorBlock segments are rendered as a compact, color-coded horizontal timeline bar with hover tooltips. | [`details_view.rs`](../youtube-gui/src/views/details_view.rs) |
| **9** | **Help Users Recognize, Diagnose, and Recover from Errors** | • Raw API, OAuth, quota, and subprocess errors are classified by `diagnose_error` into structured banners (`render_error_banner`) with a plain-English summary, actionable recovery buttons (`🔑 Re-authenticate`, `⚙ Open Settings`, `🔄 Retry`), and a collapsible `Technical Details` drawer with `📋 Copy Error`. | [`views.rs`](../youtube-gui/src/views.rs) |
| **10** | **Help and Documentation** | • Inline `ⓘ` badges (`help_badge`) next to technical controls (API Quota, RSS Fallback, Cookies from Browser, SponsorBlock) explain concepts on hover.<br>• Built-in `ℹ About & Help` view (`Alt+7`) provides an interactive Keyboard Shortcuts card, System Dependencies status check, and Troubleshooting guide. | [`views.rs`](../youtube-gui/src/views.rs), [`about_view.rs`](../youtube-gui/src/views/about_view.rs) |

---

## Global Keyboard Shortcuts Reference

`youtube-gui` processes global keyboard accelerators at the start of every frame in [`YoutubeApp::update`](../youtube-gui/src/main.rs). Shortcuts that use single keys (`Space`, `/`) automatically yield when a text input field (such as the Search box or Comment composer) has keyboard focus.

### Navigation & View Switching

| Shortcut | Action | Notes |
| :--- | :--- | :--- |
| `Alt + 1` | Open **📺 Subscriptions** view | Loads subscribed channels if not already cached |
| `Alt + 2` | Open **🆕 New Videos** feed | Displays recent uploads across subscriptions |
| `Alt + 3` | Open **📂 Playlists** view | Displays user and saved playlists |
| `Alt + 4` | Open **📁 Library** view | Scans local downloads directory |
| `Alt + 5` | Open **⏱ History** view | Displays local watch progress history |
| `Alt + 6` | Open **⚙ Settings** view | Opens configuration & account settings |
| `Alt + 7` | Open **ℹ About & Help** view | Displays shortcuts, diagnostics & help |
| `Esc` or `Alt + Left` | **Navigate Back** / Dismiss Modal | Pops the previous view from `view_stack` or cancels any open confirmation prompt |

### Search, Refresh & Persistence

| Shortcut | Action | Notes |
| :--- | :--- | :--- |
| `Ctrl + F` or `/` | **Focus Search Input** | Moves keyboard cursor directly into the top search bar (`/` works when no text field is focused) |
| `Enter` *(in Search)* | **Execute Search** | Runs the query with active search filters and records it in Recent Searches |
| `F5` or `Ctrl + R` | **Refresh Current View** | Reloads Subscriptions, New Videos, Playlists, Channel Videos, Playlist Videos, or Local Library |
| `Ctrl + S` | **Save Settings** | Immediately persists changes when viewing `⚙ Settings` |

### Media Playback

| Shortcut | Action | Notes |
| :--- | :--- | :--- |
| `Space` | **Toggle Play / Pause Audio** | Pauses or resumes the built-in `rodio` background audio player when no text input is focused |

---

## Automated Error Diagnosis & Recovery System

Instead of displaying raw Rust or HTTP error strings, [`diagnose_error`](../youtube-gui/src/views.rs) inspects failure messages and categorizes them into user-friendly [`ErrorDiagnosis`](../youtube-gui/src/views.rs) structures rendered by [`render_error_banner`](../youtube-gui/src/views.rs):

| Error Category | Trigger Patterns | Plain-Language Guidance | One-Click Recovery Action |
| :--- | :--- | :--- | :--- |
| **Authentication Expired / Missing** | `401`, `unauthenticated`, `invalid_grant`, `token`, `oauth`, `credentials` | *"Your Google sign-in session has expired or credentials are missing."* | **`🔑 Re-authenticate`** (`PendingAction::Login`) |
| **API Quota Exceeded** | `403`, `quotaExceeded`, `dailyLimitExceeded`, `quota` | *"Your daily YouTube Data API v3 quota (10,000 units) has been reached. Channel feeds will automatically use zero-quota RSS where possible."* | **`⚙ Open Settings`** (`View::Settings`) |
| **External Tool Missing (`yt-dlp` / `ffmpeg` / `mpv`)** | `yt-dlp`, `ffmpeg`, `mpv`, `vlc`, `not found`, `program not found` | *"A required external media tool (yt-dlp, ffmpeg, or media player) could not be found on your system PATH."* | **`⚙ Open Settings`** (`View::Settings`) |
| **Bot / Sign-In Verification (`yt-dlp`)** | `Sign in to confirm`, `cookies`, `age-restricted`, `HTTP Error 429` | *"YouTube requested browser verification for this stream. Configure 'Cookies from Browser' (e.g., firefox or chrome) in Settings."* | **`⚙ Configure Cookies in Settings`** (`View::Settings`) |
| **Network / Timeout Error** | `dns`, `connect`, `timed out`, `network`, `connection reset` | *"Could not reach YouTube servers. Please check your internet connection and try again."* | **Contextual `🔄 Retry`** button |

Every error banner also includes:
* A collapsible **`🔬 Technical Details`** section containing the exact raw error string for debugging.
* A **`📋 Copy Error`** button that copies the full technical message to the system clipboard.
* A **`✖ Dismiss`** button to clear the banner.

---

## Destructive Action Confirmation Safeguards

To satisfy **Heuristic #5 (Error Prevention)**, `AppState` tracks explicit confirmation states in [`youtube-gui/src/types.rs`](../youtube-gui/src/types.rs) so accidental clicks never destroy user data:

1. **Unsubscribe from Channel (`confirm_unsubscribe`):** Prompts `Confirm Unsubscribe? [Yes, Unsubscribe] [Cancel]` in Subscriptions and Channel headers.
2. **Clear All Feeds (`confirm_clear_all_feeds`):** Prompts before marking all videos in `🆕 New Videos` as dismissed. Even after dismissing individual videos, `↩ Undo Dismiss (N)` restores them in LIFO order.
3. **Delete Local Media File (`confirm_delete_file`):** Prompts `Delete "<filename>" permanently from disk?` in the `📁 Library` view before removing `.mp4` or `.mp3` files.
4. **Clear Watch History (`confirm_clear_history`):** Prompts before wiping `playback_positions.json` in the `⏱ History` view.
5. **Remove Video from Playlist (`confirm_remove_playlist_item`):** Prompts before issuing a `playlistItems.delete` (`50 quota units`) API call in `PlaylistVideos`.
6. **Disconnect Google Account (`confirm_logout`):** Prompts in `⚙ Settings` before deleting `tokencache.json`.
7. **Reset Quota Counter (`confirm_reset_quota`):** Prompts in `⚙ Settings` before zeroing out the local quota tracker.

---

## Contextual Help & Visual Status Indicators

### Inline `ⓘ` Help Badges
Technical controls across the UI render a subtle `ⓘ` badge via [`help_badge(ui, text)`](../youtube-gui/src/views.rs). Hovering over the badge explains:
* How YouTube Data API v3 quota units are consumed (`1 unit` for lists/details, `50 units` for writes, `100 units` for searches).
* How Zero-Quota RSS feed fallback (`https://www.youtube.com/feeds/videos.xml?channel_id=...`) saves API budget.
* Why browser cookies (`--cookies-from-browser`) help `yt-dlp` and `mpv` play age-restricted or rate-limited streams.
* How SponsorBlock categories (`sponsor`, `selfpromo`, `interaction`, `intro`, `outro`) work.

### Semantic Color System
Defined in [`youtube-gui/src/views.rs`](../youtube-gui/src/views.rs):
* **`COLOR_ACTIVE` (Cornflower Blue `#64B5F6`):** Active navigation tabs, selected filter chips, and informational badges.
* **`COLOR_SUCCESS` (Emerald Green `#50C878`):** Authenticated status, completed downloads, positive like ratios, and healthy API quota (`<50%`).
* **`COLOR_WARNING` (Amber Gold `#FFB74D`):** Partially watched progress badges (`⏱ Watched to MM:SS`), active buffering/downloads, and moderate API quota (`50–80%`).
* **`COLOR_DANGER` (Coral Red `#F05A5A`):** Destructive confirmation prompts, high API quota (`>80%`), and error banners.
* **`COLOR_ACTION` (Soft Purple `#B39DDB`):** Secondary media actions (`🎧 Listen`, SponsorBlock indicators).

