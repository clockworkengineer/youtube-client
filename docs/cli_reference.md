# YouTube CLI Client Reference Manual (`youtube-client`)

`youtube-client` is a versatile, high-performance command-line interface for the YouTube Data API v3. It supports interactive authentication, feed inspection, channel upload tracking, video searching, rating, playlist management, commenting, subscription operations, media downloading via `yt-dlp`, and local audio/video playback.

---

## Table of Contents

1. [Global Options & Credentials](#global-options--credentials)
2. [Command Overview Matrix](#command-overview-matrix)
3. [Detailed Subcommand Reference](#detailed-subcommand-reference)
   - [login](#1-login)
   - [subscriptions](#2-subscriptions)
   - [videos](#3-videos)
   - [search](#4-search)
   - [rate](#5-rate)
   - [playlists](#6-playlists)
   - [download](#7-download)
   - [play](#8-play)
   - [details](#9-details)
   - [channel](#10-channel)
   - [comments](#11-comments)
   - [comment-post](#12-comment-post)
   - [subscribe](#13-subscribe)
   - [unsubscribe](#14-unsubscribe)
   - [playlist-create](#15-playlist-create)
   - [playlist-delete](#16-playlist-delete)
4. [Shell Scripting & JSON Pipelining Recipes](#shell-scripting--json-pipelining-recipes)
5. [Exit Codes & Error Handling](#exit-codes--error-handling)

---

## Global Options & Credentials

All subcommands inherit the following global options. They may be passed before or after the subcommand.

| Flag / Option | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `-c, --config <FILE>` | — | `config.json` | Path to client credentials JSON file. |
| `-t, --token-cache <FILE>` | — | `tokencache.json` | Path to cached OAuth2 token file. |
| `-l, --log-file <FILE>` | `YOUTUBE_CLIENT_LOG_FILE` | `youtube-client.log` | Path to destination log file for operations, yt-dlp, and ffmpeg traces. |
| `--cookies <FILE>` | `YOUTUBE_COOKIES_FILE` | — | Path to Netscape-format `cookies.txt` file for yt-dlp/MPV. |
| `--cookies-from-browser <NAME>` | `YOUTUBE_COOKIES_FROM_BROWSER` | — | Browser name to extract cookies from (`chrome`, `firefox`, `edge`, `brave`, etc.). |
| `--client-id <ID>` | `GOOGLE_CLIENT_ID` | Embedded Default | Google OAuth2 Client ID. |
| `--client-secret <SECRET>` | `GOOGLE_CLIENT_SECRET` | Embedded Default | Google OAuth2 Client Secret. |
| `-h, --help` | — | — | Print command-line help information. |
| `-V, --version` | — | — | Print version information. |

> [!TIP]
> Credentials can also be placed in `private_config.json` in the current directory or in the OS global configuration directory (see [Configuration Guide](configuration.md)).

---

## Command Overview Matrix

| Subcommand | Purpose | Pagination Support | `--json` Output |
| :--- | :--- | :---: | :---: |
| `login` | Interactive OAuth2 browser login | — | — |
| `subscriptions` | List subscribed YouTube channels | ✅ (`--page-token`, `--all`) | ✅ |
| `videos` | List uploads for a specific channel ID | ✅ (`--page-token`) | ✅ |
| `search` | Search YouTube videos by keyword query | ✅ (`--page-token`) | ✅ |
| `rate` | Like, dislike, or clear rating on a video | — | — |
| `playlists` | List owned playlists or list playlist items | — | ✅ |
| `download` | Download video or audio via `yt-dlp` | — | — |
| `play` | Play a local media file (audio or system player) | — | — |
| `details` | Display comprehensive metrics and metadata | — | ✅ |
| `channel` | Display channel profile and cumulative stats | — | ✅ |
| `comments` | List top-level comment threads on a video | — | ✅ |
| `comment-post` | Post a new top-level comment | — | — |
| `subscribe` | Subscribe to a channel by ID | — | — |
| `unsubscribe` | Unsubscribe by subscription ID | — | — |
| `playlist-create` | Create a new user playlist | — | — |
| `playlist-delete` | Delete an existing user playlist | — | — |

---

## Detailed Subcommand Reference

### 1. `login`

Authenticates with Google OAuth2 using the Desktop Application flow. A local browser tab opens for authorization, and the resulting refresh token is saved to the token cache.

```bash
youtube-client login
```

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

Searches YouTube for videos matching a text query.

```bash
youtube-client search --query <QUERY> [OPTIONS]
```

#### Options:
* `-q, --query <STRING>`: Search query terms.
* `-l, --limit <LIMIT>`: Maximum search results (default: `20`).
* `--page-token <TOKEN>`: Resumption pagination token.
* `--json`: Output results as JSON.

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

Downloads YouTube video or audio content locally using `yt-dlp`.

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
* `--arg <EXTRA>`: Additional flags passed directly to `yt-dlp` (can be passed multiple times).

#### Examples:
```bash
# Download 1080p MP4
youtube-client download --video-id dQw4w9WgXcQ --quality 1080p

# Download audio only as MP3 into the downloads folder
youtube-client download --video-id dQw4w9WgXcQ --format mp3 --output downloads/song.mp3

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

Fetches detailed metrics and metadata for a video, including view count, like count, comment count, ISO duration, and topic tags.

```bash
youtube-client details --video-id <VIDEO_ID> [OPTIONS]
```

#### Options:
* `-v, --video-id <ID>`: Target YouTube Video ID.
* `--json`: Output structured JSON.

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

### 15. `playlist-create`

Creates a new playlist on your YouTube account.

```bash
youtube-client playlist-create --title <TITLE> [OPTIONS]
```

#### Options:
* `-t, --title <STRING>`: Title for the new playlist.
* `-d, --description <STRING>`: (Optional) Description text.

---

### 16. `playlist-delete`

Deletes a playlist owned by your account.

```bash
youtube-client playlist-delete --playlist-id <PLAYLIST_ID>
```

---

## Shell Scripting & JSON Pipelining Recipes

### 1. Extract All Subscribed Channel Titles & IDs
```bash
youtube-client subscriptions --all --json | jq -r '.[] | "\(.title) - \(.channel_id)"'
```

### 2. Search Videos and Extract Video URLs
```bash
youtube-client search --query "Rust async" --limit 5 --json | jq -r '.[].id | "https://www.youtube.com/watch?v=\(.)"'
```

### 3. Check Subscriber Count of a Channel
```bash
youtube-client channel --channel-id UC_x5XG1OV2P6uZZ5FSM9Ttw --json | jq '{title: .title, subs: .subscriber_count, views: .view_count}'
```

### 4. Batch Download MP3 Audio for Top 3 Search Results
```bash
youtube-client search --query "Synthwave mix" --limit 3 --json | jq -r '.[].id' | while read id; do
    youtube-client download --video-id "$id" --format mp3
done
```

---

## Exit Codes & Error Handling

* **`0`**: Command executed successfully.
* **`1`**: General runtime error (e.g. invalid input arguments, file not found).
* **API / Authentication Failures**:
  * If credentials are missing or invalid, instructions for configuration are displayed.
  * If the token has expired and cannot be refreshed, run `youtube-client login` to re-authenticate.
  * Rate limiting triggers automatic exponential backoff retries before reporting an error.
