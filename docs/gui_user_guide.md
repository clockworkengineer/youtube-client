# Desktop GUI User Guide (`youtube-gui`)

`youtube-gui` is a native, fast desktop application for browsing YouTube feeds, managing playlists, reading and posting comments, streaming video via external players, and listening to background audio with an integrated player.

---

## Table of Contents

1. [Starting the Application](#starting-the-application)
2. [Layout & Navigation](#layout--navigation)
3. [View Walkthroughs](#view-walkthroughs)
   - [Login View](#login-view)
   - [Subscriptions View & Real-Time Filter](#subscriptions-view--real-time-filter)
   - [New Videos Feed & Persistent Dismissal](#new-videos-feed--persistent-dismissal)
   - [Playlists Manager (Create & Delete)](#playlists-manager-create--delete)
   - [Video Details, Engagement & Comments](#video-details-engagement--comments)
   - [Settings View & Account Management](#settings-view--account-management)
   - [Downloads Manager & Progress Tracking](#downloads-manager--progress-tracking)
   - [Logs & Diagnostics](#logs--diagnostics)
4. [Integrated Audio Player Dock & Timeline Scrubber](#integrated-audio-player-dock--timeline-scrubber)
5. [Watch Progress & Resume Playback](#watch-progress--resume-playback)
6. [Video Streaming & External Media Players](#video-streaming--external-media-players)

---

## Starting the Application

### Via Cargo
```bash
cargo run --bin youtube-gui
# Or specify a custom log file destination:
cargo run --bin youtube-gui -- --log-file custom_client.log
```

### Via Installed Desktop Shortcut or Application Menu
If installed via `youtube-installer` or the Windows setup executable, launch **YouTube Client GUI** from your Start Menu (Windows) or Application Launcher (Linux).

---

## Layout & Navigation

The application consists of three primary visual zones:

1. **Top Header & Navigation Bar:**
   * View switching tabs: **Subscriptions**, **New Videos**, **Playlists**, **Downloads**, **Settings**, **Logs**, and **About**.
   * Shows active authentication badge, refresh buttons, and window controls.
2. **Main Content Canvas:**
   * Dynamic view area with smooth scrolling, grid cards, metrics badges, watch progress indicators, and interactive controls.
3. **Bottom Audio Player Dock:**
   * Persistent playback strip controlling background audio via `rodio` with interactive timeline scrubber, elapsed/total duration, volume slider, and skip buttons.

---

## View Walkthroughs

### Login View

If you do not have an active OAuth token, `youtube-gui` displays the login view:
1. Click **"Sign In with Google"**.
2. A browser window opens prompting you to approve the requested YouTube permissions.
3. Once approved, the application automatically catches the token, persists it securely to `tokencache.json`, and loads your subscriptions.

---

### Subscriptions View & Real-Time Filter

* **Real-Time Channel Filter:** A search box at the top of the view lets you type any channel name or keyword. The list dynamically narrows down matching channels as you type.
* **Channel Cards:** Each card displays the channel's thumbnail, title, and channel ID.
* **View Uploads:** Click **"View Uploads"** to inspect recent video uploads from that channel.

---

### New Videos Feed & Persistent Dismissal

The New Videos view automatically collects recent uploads across all channels you subscribe to.

* **Single Video Dismissal ("Clear"):** Click the **"✕ Clear"** button on any video card to remove it from your current feed.
* **Bulk Clearance ("Clear All Videos"):** Use the top button to clear all currently loaded videos at once.
* **Persistence:** Cleared video IDs are written atomically to `cleared_videos.json`. Cleared videos remain dismissed across application restarts.
* **Watch Progress:** Videos with watched history display a red progress bar under the thumbnail and a `⏱ Watched to MM:SS` badge.

---

### Playlists Manager (Create & Delete)

* **Browse Playlists:** View all playlists created on your YouTube account, complete with thumbnail mosaics and item counts.
* **➕ Create New Playlist:** Expand the inline playlist creation panel, enter a **Title** and optional **Description**, and click **"Create Playlist"**. The new playlist appears immediately in your list.
* **🗑 Delete Playlist:** Each user-owned playlist card includes a **"🗑 Delete"** button. Clicking it prompts for confirmation before permanently deleting the playlist from YouTube.
* **View Playlist Items:** Click on any playlist card to view its videos.

---

### Video Details, Engagement & Comments

Clicking **"Details"** on any video card opens the comprehensive Video Details view:

* **Engagement Metrics Bar:**
  * 👁 **Views:** Formatted view count (e.g., `1,420,550 views`).
  * 👍 **Likes:** Real-time like counter.
  * 👎 **Dislikes (RYD):** Restored community dislike metrics from Return YouTube Dislike (RYD) API with explanatory tooltip.
  * 💬 **Comments:** Total comments count.
  * ⏱ **Duration:** Parsed video duration (e.g., `14:25`).
* **Channel Link:** Clickable channel badge taking you directly to that channel's uploads.
* **Action Buttons:**
  * **▶ Resume (MM:SS) / ▶ Start Local Audio:** Play audio through the bottom dock with resume capability.
  * **📺 Stream Video:** Launch in preferred external player with start time offset.
  * **🌐 Watch in Browser:** Open video in default web browser.
  * **📥 Download:** Trigger background media download with real-time percentage progress bar.
* **Interactive Comments:**
  * Scroll through top-level comment threads with author names and like counts.
  * **Post Comment Form:** Enter text into the text box and click **"💬 Post Comment"** to submit a new comment directly to YouTube.

---

### Settings View & Account Management

The dedicated **Settings** view provides graphical configuration:

* **Preferred Media Player:** Select between **Auto-Detect**, **MPV**, **VLC**, or specify a custom executable path.
* **Downloads Directory:** Configure the folder where downloaded video and audio files are stored, with an in-app file browser directory picker.
* **Browser Cookies Extraction:** Select your browser (`chrome`, `firefox`, `edge`, `brave`) to automatically extract authentication cookies for `yt-dlp` and MPV.
* **Quota Budget Status Card:** Real-time visual gauge displaying your YouTube Data API v3 daily usage (searches, reads, writes) against the standard 10,000 unit budget with daily UTC rollover.
* **Import & Export Subscriptions:**
  * **Export Subscriptions to OPML:** Export all subscriptions to a standard OPML XML file saved in your downloads directory.
  * **Import Subscriptions (OPML / CSV / JSON):** Select any OPML, Google Takeout CSV, or NewPipe JSON file; the app auto-detects the format and merges channels into your active subscriptions.
* **Account Management:** Click **"Sign Out & Disconnect Account"** to safely purge cached OAuth tokens and reset the view state to Login.

---

### Downloads Manager & Progress Tracking

* Displays all media files saved in the configured `downloads/` directory.
* **Real-Time Progress:** Active downloads show a smooth percentage progress bar (`egui::ProgressBar`) with download speed and ETA.
* Play downloaded audio tracks directly via the internal audio sink.
* Launch external players or open the containing folder in your system file explorer.

---

### Logs & Diagnostics

* **In-App Logs:** Displays asynchronous background task status, API query durations, and detailed error messages if an operation fails.
* **Client Log File (`youtube-client.log`):** All background `yt-dlp` download streams and `ffmpeg` post-processing traces (such as audio extraction and container muxing) are piped directly into the client log file without spawning any intrusive console or trace windows.
* **Configurable Log Path:** Configure the log file destination via the `--log-file <PATH>` command-line parameter, the `YOUTUBE_CLIENT_LOG_FILE` environment variable, or `"log_file"` in `config.json`.

---

## Integrated Audio Player Dock & Timeline Scrubber

Located at the bottom of the application window:

```
[▶ Play / ⏸ Pause] [⏹ Stop] [⏪ 10s] [⏩ 10s]   03:45 / 12:30 ───●──────────   [🔊 Volume: ───●───── 80%] [🔇 Mute]
```

* **Controls:** Play, Pause, Stop, `⏪ 10s` backward skip, and `⏩ 10s` forward skip.
* **Timeline Scrubber:** Drag the timeline slider to seek directly to any point in the audio stream.
* **Duration Display:** Current elapsed timestamp and total media length (`MM:SS / MM:SS`).
* **Volume Slider:** Adjust volume smoothly between `0%` and `100%`.
* **Mute Toggle:** Quickly silence playback and restore to previous level with one click.
* **Background Worker:** Audio decoding runs on a dedicated hardware thread using `rodio`, ensuring audio never stutters during heavy UI rendering.

---

## Watch Progress & Resume Playback

`youtube-gui` automatically tracks playback progress:
* Watched timestamps are continuously synchronized every 200ms and saved to `%APPDATA%/youtube-client/playback_positions.json` (or `~/.config/youtube-client/playback_positions.json`).
* Video cards display a red progress bar under the thumbnail and a `⏱ Watched to MM:SS` badge.
* Action buttons offer an instant **"▶ Resume (MM:SS)"** option that starts playback from the exact watched position for both internal audio and external media players (`--start` on MPV and `--start-time` on VLC).
* Fully completed videos automatically clear their resume marker.

---

## Video Streaming & External Media Players

Clicking **"📺 Stream Video"** or a video thumbnail attempts to stream video through your preferred desktop media player:

1. **MPV (Recommended):** If `mpv` is installed, the GUI launches it with `--start <seconds>` if resuming. Cookies from the configured browser are forwarded automatically.
2. **VLC Media Player:** If MPV is absent but VLC is detected, VLC is launched with `--start-time=<seconds>`.
3. **🌐 Watch in Browser:** Click the dedicated **"🌐 Watch in Browser"** button in Video Details to open the video instantly in your default web browser (bypassing any external player bot-check errors).

> [!TIP]
> To configure cookies for MPV and yt-dlp, select your browser in the **Settings** view or specify `"cookies_from_browser": "firefox"` in `config.json`.
