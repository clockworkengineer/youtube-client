# YouTube CLI Client Reference Manual (`youtube-client`)

`youtube-client` is a versatile, high-performance command-line interface for the YouTube Data API v3. It supports interactive and headless authentication, feed inspection, channel upload tracking, filtered video searching, rating, playlist management, commenting, multi-format subscription import/export, offline media library inspection, watch history management, media downloading via `yt-dlp`, and local audio/video playback across **23 subcommands**.

---

## Table of Contents

1. [Global Options & Credentials](#global-options--credentials)
2. [Command Overview Matrix (23 Subcommands)](#command-overview-matrix-23-subcommands)
3. [Detailed Subcommand Reference](#detailed-subcommand-reference)
   - [1. login](#1-login)
   - [2. subscriptions](#2-subscriptions)
   - [3. videos](#3-videos)
   - [4. search](#4-search)
   - [5. rate](#5-rate)
   - [6. playlists](#6-playlists)
   - [7. download](#7-download)
   - [8. play](#8-play)
   - [9. details](#9-details)
   - [10. channel](#10-channel)
   - [11. comments](#11-comments)
   - [12. comment-post](#12-comment-post)
   - [13. subscribe](#13-subscribe)
   - [14. unsubscribe](#14-unsubscribe)
   - [15. export-subscriptions](#15-export-subscriptions)
   - [16. import-subscriptions](#16-import-subscriptions)
   - [17. playlist-create](#17-playlist-create)
   - [18. playlist-delete](#18-playlist-delete)
   - [19. playlist-remove-video](#19-playlist-remove-video)
   - [20. playlist-update](#20-playlist-update)
   - [21. library](#21-library)
   - [22. history](#22-history)
   - [23. completions](#23-completions)
4. [Output Formatting Engine (Table, JSON, CSV)](#output-formatting-engine-table-json-csv)
5. [Shell Scripting & JSON Pipelining Recipes](#shell-scripting--json-pipelining-recipes)
6. [Exit Codes & Error Handling](#exit-codes--error-handling)

---

## Global Options & Credentials

All subcommands inherit the following global options. They may be passed before the subcommand.

| Flag / Option | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `-c, --config <FILE>` | — | `config.json` | Path to client credentials JSON file. |
| `-t, --token-cache <FILE>` | — | `tokencache.json` | Path to cached OAuth2 token file. |
| `--api-key <KEY>` | `YOUTUBE_API_KEY` | — | Google Cloud API key for zero-OAuth public access (`search`, `videos`, `details`, `channel`, `comments`). |
| `-l, --log-file <FILE>` | `YOUTUBE_CLIENT_LOG_FILE` | `youtube-client.log` | Path to destination log file for operations, `yt-dlp`, and `ffmpeg` traces. |
| `--cookies <FILE>` | `YOUTUBE_COOKIES_FILE` | — | Path to Netscape-format `cookies.txt` file for `yt-dlp`/MPV. |
| `--cookies-from-browser <NAME>` | `YOUTUBE_COOKIES_FROM_BROWSER` | — | Browser name to extract cookies from (`chrome`, `firefox`, `edge`, `brave`, etc.). |
| `--client-id <ID>` | `GOOGLE_CLIENT_ID` | Embedded Default | Google OAuth2 Client ID. |
| `--client-secret <SECRET>` | `GOOGLE_CLIENT_SECRET` | Embedded Default | Google OAuth2 Client Secret. |
| `-h, --help` | — | — | Print command-line help information. |
| `-V, --version` | — | — | Print version information. |

> [!TIP]
> Credentials can also be placed in `private_config.json` in the current directory or in the OS global configuration directory (see [Configuration Guide](configuration.md)).
> You can also bypass OAuth2 completely for read-only actions by passing `--api-key <KEY>` or setting the `YOUTUBE_API_KEY` environment variable.

---

## Command Overview Matrix (23 Subcommands)

| # | Subcommand | Purpose | Quota Cost | Pagination Support | `--json` Output |
| :---: | :--- | :--- | :---: | :---: | :---: |
| 1 | `login` | Interactive OAuth2 browser or headless device-code login | `0 units` | — | — |
| 2 | `subscriptions` | List subscribed YouTube channels | `1 unit` | ✅ (`--page-token`, `--all`) | ✅ |
| 3 | `videos` | List uploads for a specific channel ID | `1 unit` (or `0` via RSS) | ✅ (`--page-token`) | ✅ |
| 4 | `search` | Search YouTube with sort, duration, and type filters | `100 units` | ✅ (`--page-token`) | ✅ |
| 5 | `rate` | Like, dislike, or clear rating on a video | `50 units` | — | — |
| 6 | `playlists` | List owned playlists or list playlist items | `1 unit` | — | ✅ |
| 7 | `download` | Download video or audio via `yt-dlp` with resolution/bitrate control | `0 units` | — | — |
| 8 | `play` | Play a local media file (Rodio audio or system player) | `0 units` | — | — |
| 9 | `details` | Display comprehensive metrics, RYD dislikes, and metadata | `1 unit` | — | ✅ |
| 10 | `channel` | Display channel profile and cumulative statistics | `1 unit` | — | ✅ |
| 11 | `comments` | List top-level comment threads on a video | `1 unit` | — | ✅ |
| 12 | `comment-post` | Post a new top-level comment on a video | `50 units` | — | — |
| 13 | `subscribe` | Subscribe to a channel by ID | `50 units` | — | — |
| 14 | `unsubscribe` | Unsubscribe by subscription ID | `50 units` | — | — |
| 15 | `export-subscriptions` | Export subscriptions to OPML XML, Takeout CSV, or NewPipe JSON | `1 unit / page` | ✅ (Auto-all) | — |
| 16 | `import-subscriptions` | Import subscriptions from OPML XML, Takeout CSV, or NewPipe JSON | `50 units / sub` | — | — |
| 17 | `playlist-create` | Create a new user playlist | `50 units` | — | — |
| 18 | `playlist-delete` | Delete an existing user playlist | `50 units` | — | — |
| 19 | `playlist-remove-video` | Remove a specific video item from a playlist | `50 units` | — | — |
| 20 | `playlist-update` | Update a playlist's title and/or description | `50 units` | — | — |
| 21 | `library` | Browse and filter offline downloaded audio and video files | `0 units` | — | ✅ |
| 22 | `history` | Inspect, filter, remove, or clear local watch progress history | `0 units` | — | ✅ |
| 23 | `completions` | Generate shell auto-completion scripts | `0 units` | — | — |

---

## Detailed Subcommand Reference

### 1. `login`

Authenticates with Google OAuth2 using the Desktop Application browser flow or headless device-code flow. The resulting refresh token is saved to the token cache (`tokencache.json`).

```bash
# Standard browser login (opens default web browser)
youtube-client login

# Headless / terminal device-code flow (for SSH, remote containers, or servers without a browser)
youtube-client login --device-code
```

#### Options:
* `--device-code`: Use terminal interactive / device flow authentication displaying a verification URL and code instead of launching a local browser.

---

### 2. `subscriptions`

Retrieves the channels your authenticated Google account is subscribed to.

```bash
youtube-client subscriptions [OPTIONS]
```

#### Options:
* `-l, --limit <LIMIT>`: Number of subscriptions to retrieve per page (default: `20`).
* `--page-token <TOKEN>`: Resumption token for fetching subsequent pages.
* `--all`: Automatically paginate through all subscriptions until exhausted.
* `--json`: Output raw structured JSON instead of the formatted ASCII table.

#### Examples:
```bash
# Display first 10 subscriptions in a formatted table
youtube-client subscriptions --limit 10

# Fetch all subscriptions as JSON
youtube-client subscriptions --all --json
```

---

### 3. `videos`

Lists recent video uploads from a specified channel.

```bash
youtube-client videos --channel-id <CHANNEL_ID> [OPTIONS]
```

#### Options:
* `-c, --channel-id <ID>`: The YouTube Channel ID (e.g. `UC_x5XG1OV2P6uZZ5FSM9Ttw`).
* `-l, --limit <LIMIT>`: Maximum number of videos to return (default: `20`).
* `--page-token <TOKEN>`: Resumption token for next page.
* `--json`: Output results as JSON.

---

### 4. `search`

Searches YouTube for videos, channels, or playlists matching a text query with optional sort, duration, and content type filters.

```bash
youtube-client search --query <QUERY> [OPTIONS]
```

#### Options:
* `-q, --query <STRING>`: Search query terms.
* `-l, --limit <LIMIT>`: Maximum search results (default: `20`).
* `--order <ORDER>`: Sort order (`relevance`, `date`, `view-count`, `rating`, `title`).
* `--duration <DURATION>`: Video duration filter (`any`, `short` [<4m], `medium` [4–20m], `long` [>20m]).
* `--type <CONTENT_TYPE>`: Content type filter (`video`, `channel`, `playlist`, `all`).
* `--page-token <TOKEN>`: Resumption pagination token.
* `--json`: Output results as JSON.

#### Examples:
```bash
# Search for long Rust tutorials sorted by view count
youtube-client search --query "Rust tutorial" --order view-count --duration long --limit 10
```

---

### 5. `rate`

Sets or clears the user rating on a specific video.

```bash
youtube-client rate --video-id <VIDEO_ID> --rating <RATING>
```

#### Options:
* `-v, --video-id <ID>`: Target YouTube Video ID.
* `-r, --rating <like|dislike|none>`: Rating action to apply.

---

### 6. `playlists`

Lists all playlists created/owned by the authenticated account, or lists videos inside a specific playlist.

```bash
youtube-client playlists [OPTIONS]
```

#### Options:
* `-p, --playlist-id <ID>`: (Optional) If specified, retrieves video items inside that playlist.
* `-l, --limit <LIMIT>`: Maximum playlists or items to retrieve (default: `25`).
* `--json`: Output results as JSON.

---

### 7. `download`

Downloads YouTube video or audio content locally using `yt-dlp`, with support for target video resolution and audio bitrate constraints.

```bash
youtube-client download --video-id <VIDEO_ID> [OPTIONS]
```

#### Options:
* `-v, --video-id <ID>`: YouTube Video ID (e.g., `dQw4w9WgXcQ`).
* `-o, --output <PATH>`: Custom output filename or directory. Default: `<video_id>.<ext>`.
* `-f, --format <FORMAT>`: Format selector:
  * `mp4`: Best compatible MP4 video (default).
  * `mp3`: Extract audio stream and encode as MP3.
  * `bestaudio`: Best audio-only container.
  * Custom string passed directly to `yt-dlp -f`.
* `-q, --quality <QUALITY>`: Max video height constraint (e.g., `1080p`, `720p`).
* `--resolution <RESOLUTION>`: Target video resolution (`best`, `2160p`, `1440p`, `1080p`, `720p`, `480p`, `360p`).
* `--audio-quality <KBPS>`: Target audio bitrate in kbps (`best`, `320`, `256`, `192`, `128`).
* `--arg <EXTRA>`: Additional flags passed directly to `yt-dlp` (can be passed multiple times).

#### Examples:
```bash
# Download 1080p MP4
youtube-client download --video-id dQw4w9WgXcQ --resolution 1080p

# Download 320 kbps MP3 audio into the downloads folder
youtube-client download --video-id dQw4w9WgXcQ --format mp3 --audio-quality 320 --output downloads/song.mp3

# Pass custom yt-dlp rate limit argument
youtube-client download --video-id dQw4w9WgXcQ --arg "--limit-rate" --arg "5M"
```

---

### 8. `play`

Plays a locally downloaded audio or video file.

```bash
youtube-client play --file <PATH> [OPTIONS]
```

#### Options:
* `-f, --file <PATH>`: Path to the media file.
* `-s, --system`: Launch the system default media player instead of decoding audio locally via Rodio.

---

### 9. `details`

Fetches detailed metrics and metadata for a video, including view count, like count, community dislike count (via Return YouTube Dislike API integration), comment count, ISO duration, and topic tags.

```bash
youtube-client details --video-id <VIDEO_ID> [OPTIONS]
```

#### Options:
* `-v, --video-id <ID>`: Target YouTube Video ID.
* `--json`: Output structured JSON (includes `dislike_count` field).

---

### 10. `channel`

Fetches public channel statistics, subscriber counts, total videos, and view metrics.

```bash
youtube-client channel --channel-id <CHANNEL_ID> [OPTIONS]
```

#### Options:
* `-c, --channel-id <ID>`: YouTube Channel ID.
* `--json`: Output structured JSON.

---

### 11. `comments`

Fetches top-level comment threads on a video.

```bash
youtube-client comments --video-id <VIDEO_ID> [OPTIONS]
```

#### Options:
* `-v, --video-id <ID>`: Target YouTube Video ID.
* `-l, --limit <LIMIT>`: Maximum number of comments to list (default: `20`).
* `--json`: Output structured JSON.

---

### 12. `comment-post`

Posts a new top-level comment to a YouTube video.

```bash
youtube-client comment-post --video-id <VIDEO_ID> --text <COMMENT_TEXT>
```

#### Options:
* `-v, --video-id <ID>`: Target YouTube Video ID.
* `-t, --text <TEXT>`: Comment text string.

---

### 13. `subscribe`

Subscribes the authenticated user account to a channel.

```bash
youtube-client subscribe --channel-id <CHANNEL_ID>
```

---

### 14. `unsubscribe`

Removes a channel subscription by its unique subscription ID.

```bash
youtube-client unsubscribe --subscription-id <SUBSCRIPTION_ID>
```

---

### 15. `export-subscriptions`

Exports all of your YouTube subscriptions to an OPML 2.0 XML, Google Takeout CSV, or NewPipe JSON file.

```bash
youtube-client export-subscriptions --output <PATH> [OPTIONS]
```

#### Options:
* `-f, --format <opml|csv|json>`: Output file format (default: `opml`).
* `-o, --output <PATH>`: Destination file path.

#### Examples:
```bash
# Export subscriptions as OPML XML for FreeTube / RSS readers
youtube-client export-subscriptions --output subscriptions.opml

# Export subscriptions as NewPipe JSON
youtube-client export-subscriptions --format json --output newpipe_subs.json
```

---

### 16. `import-subscriptions`

Imports YouTube subscriptions from an OPML XML, Google Takeout CSV, or NewPipe JSON backup file and subscribes your account to each channel.

```bash
youtube-client import-subscriptions --file <PATH> [OPTIONS]
```

#### Options:
* `-f, --format <auto|opml|csv|json>`: Input format strategy (default: `auto`, which automatically detects the format from content and extension).
* `-i, --file, --input <PATH>`: Path to the backup file to import.

---

### 17. `playlist-create`

Creates a new playlist on your YouTube account.

```bash
youtube-client playlist-create --title <TITLE> [OPTIONS]
```

#### Options:
* `-t, --title <STRING>`: Title for the new playlist.
* `-d, --description <STRING>`: (Optional) Description text.

---

### 18. `playlist-delete`

Deletes a playlist owned by your account.

```bash
youtube-client playlist-delete --playlist-id <PLAYLIST_ID>
```

---

### 19. `playlist-remove-video`

Removes a specific video item from a playlist using its unique playlist item ID.

```bash
youtube-client playlist-remove-video --playlist-item-id <PLAYLIST_ITEM_ID>
```

#### Options:
* `-p, --playlist-item-id <ID>`: The unique playlist item ID returned in playlist item listings.

---

### 20. `playlist-update`

Updates an existing playlist's title and/or description.

```bash
youtube-client playlist-update --playlist-id <PLAYLIST_ID> --title <NEW_TITLE> [OPTIONS]
```

#### Options:
* `-p, --playlist-id <ID>`: Target playlist ID to modify.
* `-t, --title <STRING>`: New title for the playlist.
* `-d, --description <STRING>`: Optional new description for the playlist.

---

### 21. `library`

Scans and lists downloaded offline audio and video media files in your downloads directory with file sizes and media categories.

```bash
youtube-client library [OPTIONS]
```

#### Options:
* `-d, --dir <PATH>`: Optional directory path to scan (defaults to configured `downloads_dir` or `./downloads`).
* `--audio-only`: Filter for audio files only (`.mp3`, `.m4a`, `.wav`, `.ogg`, `.flac`).
* `--video-only`: Filter for video files only (`.mp4`, `.mkv`, `.webm`).
* `--json`: Output results as formatted JSON.

#### Examples:
```bash
# List all downloaded files in a table
youtube-client library

# List only downloaded MP3/M4A audio files as JSON
youtube-client library --audio-only --json
```

---

### 22. `history`

Views, limits, removes entries from, or clears your local persistent video watch progress history (`playback_positions.json`).

```bash
youtube-client history [OPTIONS]
```

#### Options:
* `-l, --limit <COUNT>`: Limit the number of history entries displayed (sorted most-recently watched first).
* `-r, --remove <VIDEO_ID>`: Remove a specific video ID from watch history.
* `--clear`: Clear the entire watch progress history file.
* `-f, --file <PATH>`: Optional custom path to `playback_positions.json`.
* `--json`: Output watch history entries as formatted JSON.

#### Examples:
```bash
# View your 15 most recently watched videos and resume timestamps
youtube-client history --limit 15

# Remove a single video from watch history
youtube-client history --remove dQw4w9WgXcQ

# Clear all watch history
youtube-client history --clear
```

---

### 23. `completions`

Generates tab-completion scripts for your preferred shell.

```bash
youtube-client completions <SHELL>
```

#### Supported Shells:
* `bash`: `youtube-client completions bash > ~/.local/share/bash-completion/completions/youtube-client`
* `zsh`: `youtube-client completions zsh > ~/.zfunc/_youtube-client`
* `fish`: `youtube-client completions fish > ~/.config/fish/completions/youtube-client.fish`
* `powershell`: `youtube-client completions powershell >> $PROFILE`
* `elvish`: `youtube-client completions elvish`

---

## 4. Output Formatting Engine (Table, JSON, CSV)

`youtube-client` utilizes an extensible [`OutputFormatter<T>`](../youtube-client/src/formatters.rs) engine that cleanly separates API data retrieval from terminal presentation:

### Available Formatting Strategies

1. **Table / Card Formatter (Default):**
   - Automatically formats tabular data (`subscriptions`, `videos`, `search`, `playlists`, `comments`, `library`, `history`) with aligned columns, headers, and separator rules.
   - Formats single items (`details`, `channel`) into clear ASCII summary cards.
   - Appends continuation pagination instructions automatically if a `next_page_token` exists.
2. **JSON Formatter (`--json`):**
   - Serializes complete domain models or paginated `Page<T>` containers into formatted, pretty-printed JSON.
   - Ideal for piping into `jq`, automated testing, or scripting.
3. **CSV Formatter:**
   - Available via the `CsvFormatter` strategy in `youtube_client::formatters`, generating RFC-4180 compliant comma-separated values with quotation escaping for spreadsheets and data science pipelines.

---

## 5. Shell Scripting & JSON Pipelining Recipes

### 1. Extract All Subscribed Channel Titles & IDs
```bash
youtube-client subscriptions --all --json | jq -r '.[] | "\(.title) - \(.channel_id)"'
```

### 2. Search Videos with Filters and Extract Video URLs
```bash
youtube-client search --query "Rust async" --order view-count --duration medium --limit 5 --json | jq -r '.[].id | "https://www.youtube.com/watch?v=\(.)"'
```

### 3. Check Subscriber Count of a Channel
```bash
youtube-client channel --channel-id UC_x5XG1OV2P6uZZ5FSM9Ttw --json | jq '{title: .title, subs: .subscriber_count, views: .view_count}'
```

### 4. Batch Download 320kbps MP3 Audio for Top 3 Search Results
```bash
youtube-client search --query "Synthwave mix" --limit 3 --json | jq -r '.[].id' | while read id; do
    youtube-client download --video-id "$id" --format mp3 --audio-quality 320
done
```

### 5. Zero-OAuth Querying with an API Key (No Browser or Login Required)
```bash
# Query YouTube search directly using an API key
youtube-client --api-key AIzaSyYourApiKeyHere search --query "Rust async" --limit 5

# Or export via environment variable
export YOUTUBE_API_KEY="AIzaSyYourApiKeyHere"
youtube-client details --video-id dQw4w9WgXcQ --json | jq '{title, views: .view_count, dislikes: .dislike_count}'
```

---

## 6. Exit Codes & Error Handling

* **`0`**: Command executed successfully.
* **`1`**: General runtime error (e.g. invalid input arguments, file not found).
* **API / Authentication Failures**:
  * If credentials are missing or invalid, instructions for configuration are displayed.
  * If the token has expired and cannot be refreshed, run `youtube-client login` to re-authenticate.
  * Rate limiting triggers automatic exponential backoff retries before reporting an error.
