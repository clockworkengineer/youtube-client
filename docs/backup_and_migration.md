# Subscription Backup, Migration & Data Portability Guide (v0.2.0)

This guide explains how to export, import, back up, and migrate your YouTube subscriptions, offline media library, and watch history across platforms (Google Takeout, FreeTube, NewPipe, RSS readers, and other YouTube accounts) using both the `youtube-client` CLI and `youtube-gui` desktop application.

---

## Table of Contents

1. [Supported Subscription Formats](#supported-subscription-formats)
2. [CLI Subscription Export & Import Workflows](#cli-subscription-export--import-workflows)
   - [Exporting Subscriptions (`export-subscriptions`)](#1-exporting-subscriptions-export-subscriptions)
   - [Importing Subscriptions (`import-subscriptions`)](#2-importing-subscriptions-import-subscriptions)
3. [GUI Subscription Export & Import Workflows](#gui-subscription-export--import-workflows)
4. [Obtaining Exports from Other Platforms](#obtaining-exports-from-other-platforms)
   - [Google Takeout (`subscriptions.csv`)](#google-takeout-subscriptionscsv)
   - [NewPipe (`.json`)](#newpipe-json)
   - [FreeTube & RSS Readers (`.opml` / `.xml`)](#freetube--rss-readers-opml--xml)
5. [Backing Up & Auditing Local Media & Watch History](#backing-up--auditing-local-media--watch-history)

---

## Supported Subscription Formats

The workspace includes a pluggable [`SubscriptionFormatRegistry`](../youtube-client-lib/src/importers.rs) in `youtube-client-lib` that automatically detects, parses, and serializes three industry-standard subscription formats:

| Format Name | Strategy Struct | Format Flag (`-f`) | File Extensions | Compatible Applications | Export Supported | Import Supported |
| :--- | :--- | :---: | :---: | :--- | :---: | :---: |
| **OPML 2.0 XML** | [`OpmlFormat`](../youtube-client-lib/src/importers.rs) | `opml` | `.opml`, `.xml` | FreeTube, NewPipe, Thunderbird, Feedly, Inoreader, NetNewsWire | ✓ Yes | ✓ Yes |
| **Google Takeout CSV** | [`TakeoutCsvFormat`](../youtube-client-lib/src/importers.rs) | `csv` | `.csv` | Google Takeout (`subscriptions.csv`) | ✓ Yes | ✓ Yes |
| **NewPipe JSON** | [`NewPipeJsonFormat`](../youtube-client-lib/src/importers.rs) | `json` | `.json` | NewPipe (Android), PipePipe, LibreTube | ✓ Yes | ✓ Yes |

Each parsed channel produces a normalized [`SubscriptionItem`](../youtube-client-lib/src/importers.rs) containing:
* `channel_id`: The canonical 24-character YouTube channel ID (`UC...`).
* `title`: The human-readable channel name.
* Public YouTube Atom RSS feed URL (`https://www.youtube.com/feeds/videos.xml?channel_id=UC...`).

---

## CLI Subscription Export & Import Workflows

### 1. Exporting Subscriptions (`export-subscriptions`)

To export all channels from your authenticated YouTube account into OPML XML, Google Takeout CSV, or NewPipe JSON:

```bash
# Export to OPML 2.0 XML (default format)
youtube-client export-subscriptions --output my_subscriptions.opml

# Export to Google Takeout CSV format
youtube-client export-subscriptions --format csv --output subscriptions.csv

# Export to NewPipe JSON backup format
youtube-client export-subscriptions --format json --output newpipe_backup.json
```

* **How it works:** Automatically paginates through your entire subscription list (`50 channels` per page) and serializes every channel using the selected format strategy in [`SubscriptionFormatRegistry`](../youtube-client-lib/src/importers.rs).
* **API Quota Cost:** `1 unit` per page of 50 subscriptions (e.g., `4 units` for 200 subscriptions).

#### Example Generated OPML Output:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.0">
  <head>
    <title>YouTube Subscriptions Export</title>
  </head>
  <body>
    <outline text="YouTube Subscriptions" title="YouTube Subscriptions">
      <outline type="rss" text="Rust" title="Rust" xmlUrl="https://www.youtube.com/feeds/videos.xml?channel_id=UCaYhcUwRBNscFNUKTjgPFiA" />
    </outline>
  </body>
</opml>
```

---

### 2. Importing Subscriptions (`import-subscriptions`)

To import and subscribe your authenticated YouTube account to channels from an OPML XML, Google Takeout CSV, or NewPipe JSON file:

```bash
# Auto-detect format (OPML, CSV, or JSON) and import subscriptions
youtube-client import-subscriptions --file subscriptions.csv

# Import a NewPipe JSON backup using the short -i flag
youtube-client import-subscriptions -i newpipe-subscriptions.json

# Explicitly specify the input format
youtube-client import-subscriptions --format opml --file freetube.opml
```

* **API Quota Cost Warning:** The YouTube Data API v3 charges **`50 units`** for each `subscriptions.insert` write call. With the default daily limit of `10,000 units`, you can subscribe to at most **~190–200 channels per day**.
* **Best Practice for Large Imports (>190 channels):**
  If migrating hundreds of channels across accounts, either split your CSV/OPML file into batches of ~180 channels across consecutive days (quota resets daily at midnight UTC) or import the OPML in `youtube-gui` / an RSS reader for zero-quota browsing.

---

## GUI Subscription Export & Import Workflows

You can also export and import subscriptions directly from the desktop interface:

1. Launch `youtube-gui` and open the **⚙ Settings** tab (`Alt + 6`).
2. Scroll to the **📦 Subscriptions Backup (OPML / CSV / JSON)** section:
   * **To Export:** Click **`📤 Export Subscriptions to OPML...`** to serialize your subscriptions to an OPML XML file in your downloads directory.
   * **To Import:** Click **`📥 Import Subscriptions (OPML / CSV / JSON)...`** and select your `.opml`, `.xml`, `.csv`, or `.json` file.
     * The GUI automatically detects the file format, merges the imported channels into your active subscriptions list, and displays a toast notification reporting how many channels were imported.

---

## Obtaining Exports from Other Platforms

### Google Takeout (`subscriptions.csv`)
1. Visit [Google Takeout](https://takeout.google.com/).
2. Click **Deselect all**, then scroll to **YouTube and YouTube Music** and check the box.
3. Click **All YouTube data included**, deselect everything except **subscriptions**, and click **OK**.
4. Create the export, download the `.zip` archive, and extract `Takeout/YouTube and YouTube Music/subscriptions/subscriptions.csv`.
5. Pass `subscriptions.csv` directly to `youtube-client import-subscriptions --file subscriptions.csv` or select it in `youtube-gui`.

### NewPipe (`.json`)
1. In the NewPipe Android app, open the **Subscriptions** tab.
2. Tap the three-dot menu → **Export to** → **File**, which saves a `.json` file containing a `"subscriptions"` array of `https://www.youtube.com/channel/UC...` URLs.
3. Pass that `.json` file directly to `youtube-client import-subscriptions --file newpipe.json` or import it in `youtube-gui`.

### FreeTube & RSS Readers (`.opml` / `.xml`)
1. In FreeTube, go to **Settings** → **Data Settings** → **Export Subscriptions** and choose **OPML (`.opml`)**.
2. Pass the `.opml` file to `youtube-client import-subscriptions --file freetube.opml` or import it in `youtube-gui`.

---

## Backing Up & Auditing Local Media & Watch History

In addition to channel subscriptions, `youtube-client` stores all local watch progress and offline downloads in standard, portable formats inside your configuration and downloads directories.

### Auditing & Exporting Watch History (`history`)
Your video resume positions and last-watched timestamps are stored in `playback_positions.json` inside the configuration directory (`%APPDATA%\youtube-client\playback_positions.json` on Windows, `~/.config/youtube-client/playback_positions.json` on Linux).

```bash
# View watch history in a terminal table
youtube-client history

# Limit displayed history entries to the 10 most recent videos
youtube-client history --limit 10

# Remove a single video ID from watch history
youtube-client history --remove dQw4w9WgXcQ

# Export full watch history as JSON for backup
youtube-client history --json > watch_history_backup.json
```

### Auditing Offline Downloaded Media (`library`)
Your downloaded `.mp4` videos and `.mp3` audio tracks reside in your configured `downloads_dir` (default `./downloads`):

```bash
# List all downloaded audio and video files with sizes
youtube-client library

# Filter only audio files (.mp3, .m4a, .wav, .ogg, .flac)
youtube-client library --audio-only

# Filter only video files (.mp4, .mkv, .webm)
youtube-client library --video-only

# Export a JSON manifest of your offline media library
youtube-client library --json > media_library_manifest.json
```

### Portable State Files Summary

To migrate your complete local client state to a new machine, copy the following files from your configuration directory:

| File Name | Purpose | Sensitive? |
| :--- | :--- | :---: |
| `config.json` | User preferences (player, resolution, bitrate, SponsorBlock, downloads path) | No (unless `client_secret` is embedded) |
| `playback_positions.json` | Per-video watch progress (`position_secs`, `duration_secs`, `title`, `updated_at`) | No |
| `cleared_videos.json` | Set of video IDs dismissed from the `🆕 New Videos` feed | No |
| `api_quota.json` | Current day's API quota counter | No |
| `cache.json` | Persistent TTL metadata cache | No |
| `tokencache.json` | Cached OAuth2 refresh and access tokens (`0600` on Unix) | **Yes (Keep Private)** |
