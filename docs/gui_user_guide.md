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
   - [Downloads Manager](#downloads-manager)
   - [Logs & Diagnostics](#logs--diagnostics)
4. [Integrated Audio Player Bar](#integrated-audio-player-bar)
5. [Video Streaming & External Media Players](#video-streaming--external-media-players)

---

## Starting the Application

### Via Cargo
```bash
cargo run --bin youtube-gui
# Or specify a custom log file destination:
cargo run --bin youtube-gui -- --log-file custom_client.log
```

### Via Installed Desktop Shortcut or Application Menu
If installed via `youtube-installer`, launch **YouTube Client GUI** from your Start Menu (Windows) or Application Launcher (Linux).

---

## Layout & Navigation

The application consists of three primary visual zones:

1. **Top Header & Navigation Bar:**
   * Contains view switching tabs: **Subscriptions**, **New Videos**, **Playlists**, **Downloads**, **Logs**, and **About**.
   * Shows authentication status and quick refresh controls.
2. **Main Content Canvas:**
   * Dynamic view area with smooth scrolling, grid cards, metrics badges, and interactive controls.
3. **Bottom Audio Player Dock:**
   * Persistent playback strip controlling background audio via `rodio` with volume and mute toggles.

---

## View Walkthroughs

### Login View

If you do not have an active OAuth token, `youtube-gui` displays the login view:
1. Click **"Sign In with Google"**.
2. A browser window opens prompting you to approve the requested YouTube permissions.
3. Once approved, the application automatically catches the token, persists it to `tokencache.json`, and loads your subscriptions.

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
  * 💬 **Comments:** Total comments count.
  * ⏱ **Duration:** Parsed video duration (e.g., `14:25`).
* **Channel Link:** Clickable channel badge taking you directly to that channel's uploads.
* **Topic Tags:** Clickable or viewable tags associated with the video.
* **Interactive Comments:**
  * Scroll through top-level comment threads with author names and like counts.
  * **Post Comment Form:** Enter text into the text box and click **"💬 Post Comment"** to submit a new comment directly to YouTube.

---

### Downloads Manager

* Displays all media files saved in the configured `downloads/` directory.
* Play downloaded audio tracks directly via the internal audio sink.
* Launch external players or open the containing folder in your system file explorer.

---

### Logs & Diagnostics

* **In-App Logs:** Displays asynchronous background task status, API query durations, and detailed error messages if an operation fails.
* **Client Log File (`youtube-client.log`):** All background `yt-dlp` download streams and `ffmpeg` post-processing traces (such as audio extraction and container muxing) are piped directly into the client log file without spawning any intrusive console or trace windows.
* **Configurable Log Path:** Configure the log file destination via the `--log-file <PATH>` command-line parameter, the `YOUTUBE_CLIENT_LOG_FILE` environment variable, or `"log_file"` in `config.json`.

---

## Integrated Audio Player Bar

Located at the bottom of the window:

```
[▶ Play / ⏸ Pause] [⏹ Stop]   Title: Song Name - Artist   [🔊 Volume: ───●───── 80%] [🔇 Mute]
```

* **Controls:** Play, Pause, and Stop local audio decoding.
* **Volume Slider:** Adjust volume smoothly between `0%` and `100%`.
* **Mute Toggle:** Quickly silence playback and restore to previous level with one click.
* **Background Worker:** Audio decoding runs on a dedicated hardware thread using `rodio`, ensuring audio never stutters during heavy UI rendering.

---

## Video Streaming & External Media Players

Clicking **"📺 Stream Video"** or a video thumbnail attempts to stream video through your preferred desktop media player:

1. **MPV (Recommended):** If `mpv` is installed, the GUI launches it. If `cookies_file` or `cookies_from_browser` is defined in configuration, they are automatically forwarded to MPV.
2. **VLC Media Player:** If MPV is absent but VLC is detected, VLC is launched.
3. **🌐 Watch in Browser:** Click the dedicated **"🌐 Watch in Browser"** button in Video Details to open the video instantly in your default web browser (bypassing any external player bot-check errors).

> [!TIP]
> To configure cookies for MPV and yt-dlp, specify `"cookies_from_browser": "firefox"` or `"cookies_file": "path/to/cookies.txt"` in `config.json` (see [Configuration Guide](configuration.md)).
