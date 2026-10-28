# Configuration, State & Secrets Management Guide (v0.3.0)

This document defines how the YouTube Client ecosystem manages API credentials, local settings, media player paths, default download resolution/bitrate, SponsorBlock preferences, window geometry, persistent session states, and security tokens.

---

## 1. Centralized Application Data Paths

All client configuration, caches, and persistent state files are stored in standard OS application data directories resolved via [`resolve_app_data_path(file_name)`](file:///c:/Projects/youtube-client/youtube-client-lib/src/config.rs):

* **Windows:** `%APPDATA%\youtube-client\` (e.g. `C:\Users\<User>\AppData\Roaming\youtube-client\`)
* **Linux / BSD:** `${XDG_CONFIG_HOME:-~/.config}/youtube-client/`
* **macOS:** `~/Library/Application Support/youtube-client/`

Temporary audio stream files extracted by the GUI's **`🎧 Listen`** feature are cached in the OS temporary directory:
* **Windows:** `%TEMP%\youtube-client-audio\` (e.g. `C:\Users\<User>\AppData\Local\Temp\youtube-client-audio\`)
* **Linux / macOS:** `/tmp/youtube-client-audio/`

---

## 2. Configuration Resolution Precedence

When initializing a client connection or starting the GUI, configuration settings are resolved using a **strict 6-tier fallback hierarchy**:

```mermaid
graph TD
    T1["1. Command-Line Arguments<br/>(--client-id, --client-secret, --api-key, --config)"] -->|If Absent| T2["2. Environment Variables<br/>(GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, YOUTUBE_API_KEY)"]
    T2 -->|If Absent| T3["3. Local private_config.json<br/>(Ignored by git for secret safety)"]
    T3 -->|If Absent| T4["4. Local config.json<br/>(Workspace root or current working dir)"]
    T4 -->|If Absent| T5["5. OS Global Configuration File<br/>(%APPDATA%, ~/.config, ~/Library/...)"]
    T5 -->|If Absent| T6["6. Embedded Compile-Time Defaults<br/>(DEFAULT_GOOGLE_CLIENT_ID)"]
```

### Precedence Details

1. **Command-Line Arguments**: Flags passed directly to `youtube-client` (e.g. `--client-id "XYZ" --client-secret "ABC"`, `--api-key "AIza..."`, `--log-file "custom.log"`).
2. **Environment Variables**:
   * `GOOGLE_CLIENT_ID`: Overrides the Google OAuth2 Client ID.
   * `GOOGLE_CLIENT_SECRET`: Overrides the Google OAuth2 Client Secret.
   * `YOUTUBE_API_KEY`: Overrides the Google Cloud API Key for zero-OAuth read operations.
   * `YOUTUBE_CLIENT_LOG_FILE`: Overrides the destination client log file path.
   * `YOUTUBE_COOKIES_FILE`: Overrides the path to a Netscape `cookies.txt` file.
   * `YOUTUBE_COOKIES_FROM_BROWSER`: Overrides browser cookie extraction (`chrome`, `firefox`, `edge`, `brave`).
3. **Local `private_config.json`**: Checked in the current working directory. This file is explicitly listed in `.gitignore` to prevent accidental credential commits.
4. **Local `config.json`**: Fallback configuration file in the current working directory.
5. **OS Global User Configuration Directory**:
   * **Windows**: `%APPDATA%\youtube-client\config.json`
   * **macOS**: `~/Library/Application Support/youtube-client/config.json`
   * **Linux / BSD**: `${XDG_CONFIG_HOME:-~/.config}/youtube-client/config.json`
6. **Compile-Time Defaults**: Built-in OAuth credentials embedded at compile time via `option_env!("DEFAULT_GOOGLE_CLIENT_ID")` and `option_env!("DEFAULT_GOOGLE_CLIENT_SECRET")`. Default log file falls back to `youtube-client.log`.

---

## 3. Configuration File Schema (`config.json`)

Configuration files (`config.json` or `private_config.json`) map directly to [`Config`](file:///c:/Projects/youtube-client/youtube-client-lib/src/config.rs#L10-L43) and support dynamic in-app updating via the GUI Settings view (`Alt+6`, `Ctrl+S`):

```json
{
  "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
  "client_secret": "YOUR_CLIENT_SECRET",
  "api_key": null,
  "player_path": "C:\\Program Files\\mpv\\mpv.exe",
  "downloads_dir": "C:\\Users\\User\\Downloads\\YouTube",
  "log_file": "C:\\Users\\User\\AppData\\Roaming\\youtube-client\\youtube-client.log",
  "cookies_file": "C:\\Users\\User\\cookies.txt",
  "cookies_from_browser": "firefox",
  "volume": 0.85,
  "preferred_resolution": "1080p",
  "preferred_audio_bitrate": "320",
  "sponsorblock_enabled": true,
  "sponsorblock_categories": [
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro"
  ],
  "window_pos": [150.0, 100.0],
  "window_size": [1280.0, 800.0],
  "window_maximized": false
}
```

### Complete Field Definitions (`Config`)

| Property | Type | Required | Default | Description |
| :--- | :---: | :---: | :---: | :--- |
| `client_id` | `Option<String>` | No* | Embedded | Google OAuth2 Desktop Client ID. |
| `client_secret` | `Option<String>` | No* | Embedded | Google OAuth2 Desktop Client Secret. |
| `api_key` | `Option<String>` | No | `None` | Google Cloud API key for unauthenticated read operations (`search`, `videos`, `details`, `channel`, `comments`). |
| `player_path` | `Option<String>` | No | Auto-detect | Absolute path or binary name of preferred external media player (`mpv`, `vlc`, custom executable). |
| `downloads_dir` | `Option<String>` | No | `./downloads` | Default destination folder for downloaded video and audio files. |
| `log_file` | `Option<String>` | No | `youtube-client.log` | Destination path for client logs, `yt-dlp` progress, and `ffmpeg` traces. |
| `cookies_file` | `Option<String>` | No | `None` | Path to Netscape-format `cookies.txt` file for authenticated stream extraction. |
| `cookies_from_browser` | `Option<String>` | No | `None` | Browser name to extract cookies from (`chrome`, `firefox`, `edge`, `brave`, `opera`, `vivaldi`). |
| `volume` | `Option<f32>` | No | `1.0` | Persistent audio dock volume level (`0.0` muted to `1.0` max). |
| `preferred_resolution` | `Option<String>` | No | `"best"` | Default target video download resolution (`"best"`, `"2160p"`, `"1440p"`, `"1080p"`, `"720p"`, `"480p"`, `"360p"`). |
| `preferred_audio_bitrate` | `Option<String>` | No | `"best"` | Default target MP3 audio bitrate in kbps (`"best"`, `"320"`, `"256"`, `"192"`, `"128"`). |
| `sponsorblock_enabled` | `Option<bool>` | No | `true` | Enables automatic SponsorBlock segment fetching and MPV segment skipping. |
| `sponsorblock_categories` | `Option<Vec<String>>` | No | `["sponsor", "intro", "outro", "selfpromo", "interaction"]` | Active SponsorBlock categories to skip and highlight on the video timeline. |
| `window_pos` | `Option<[f32; 2]>` | No | OS Default | Last remembered window screen coordinates `[x, y]`. |
| `window_size` | `Option<[f32; 2]>` | No | `[1200.0, 820.0]` | Last remembered window logical dimensions `[width, height]`. |
| `window_maximized` | `Option<bool>` | No | `false` | Whether the GUI window was maximized on last exit. |

*\* If not supplied in the file, credentials fall back to environment variables or embedded defaults.*

---

## 4. Persistent State Files Catalog

The YouTube Client suite uses atomic sibling writes ([`write_json_atomically`](file:///c:/Projects/youtube-client/youtube-client-lib/src/utils.rs)) to manage persistent states without file corruption:

| File Name | Purpose | Location |
| :--- | :--- | :--- |
| **`config.json`** | Settings, preferred player, resolution/bitrate defaults, SponsorBlock settings, and window geometry. | `%APPDATA%/youtube-client/config.json` |
| **`tokencache.json`** | OAuth2 refresh and access token records with strict owner-only permissions. | `%APPDATA%/youtube-client/tokencache.json` |
| **`cache.json`** | Disk-backed TTL metadata cache for videos (30m) and channels (2h). | `%APPDATA%/youtube-client/cache.json` |
| **`thumbnails/`** | Disk-cached video and channel thumbnail JPEGs/PNGs. | `%APPDATA%/youtube-client/thumbnails/` |
| **`api_quota.json`** | Daily YouTube Data API v3 unit usage tracker with daily UTC rollover. | `%APPDATA%/youtube-client/api_quota.json` |
| **`playback_positions.json`** | Per-video watched timestamps (`position_secs`, `duration_secs`, `title`, `updated_at`). | `%APPDATA%/youtube-client/playback_positions.json` |
| **`cleared_videos.json`** | Dismissed video ID hashes for the `🆕 New Videos` feed. | `%APPDATA%/youtube-client/cleared_videos.json` |
| **`youtube-client.log`** | Background `yt-dlp` logs, `ffmpeg` traces, and player diagnostics. | `%APPDATA%/youtube-client/youtube-client.log` |

---

## 5. Token Cache Security & Verification

### File Permissions
Because `tokencache.json` contains active refresh tokens granting access to your YouTube account:
* `youtube-client-lib` automatically invokes [`secure_sensitive_file`](file:///c:/Projects/youtube-client/youtube-client-lib/src/config.rs) on Unix platforms to enforce `0600` permissions (read/write only by owner).
* Never share or commit `tokencache.json` or `private_config.json` to source control.

### Scope Verification
Whenever a client starts, `check_token_cache_scopes` inspects the cached token scopes against the application's required scopes (`YOUTUBE_SCOPES`):
* `https://www.googleapis.com/auth/youtube`
* `https://www.googleapis.com/auth/youtube.force-ssl`
* `https://www.googleapis.com/auth/youtube.readonly`

If your cached token was created under older or insufficient scopes (e.g. read-only), delete `tokencache.json` (or click **"Sign Out & Disconnect Account"** in the Settings view) and re-authenticate.
