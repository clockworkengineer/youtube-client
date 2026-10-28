# `youtube-client-lib` (v0.3.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Crate](https://img.shields.io/badge/crates.io-v0.3.0-orange.svg)](https://crates.io/crates/youtube-client-lib)
[![Documentation](https://docs.rs/youtube-client-lib/badge.svg)](https://docs.rs/youtube-client-lib)

A modular, asynchronous Rust library for interacting with the YouTube Data API v3, managing subscriptions, browsing uploaded videos and playlists, querying SponsorBlock skip segments, controlling MPV over IPC, streaming audio via Rodio, and downloading media streams via `yt-dlp`.

---

## Features

* **SOLID Clean Architecture:** Engineered around segregated service traits (`VideoService`, `SubscriptionService`, `PlaylistService`, `CommentService`, `MediaDownloader`) and a unified `YoutubeBackend` adapter.
* **Pluggable Media Players:** Unified [`MediaPlayer`](src/player.rs) trait and auto-detecting [`PlayerRegistry`](src/player.rs) supporting MPV (with `--force-window=immediate` and native `edl://` SponsorBlock URLs), VLC, IINA, custom binaries, and system defaults.
* **SponsorBlock & MPV IPC Integration:** Crowd-sourced skip segment lookups ([`sponsorblock.rs`](src/sponsorblock.rs)) and bi-directional MPV IPC control over Windows named pipes and Unix sockets ([`mpv_ipc.rs`](src/mpv_ipc.rs)).
* **Quality-Controlled Downloads & Cooperative Cancellation:** [`DownloadOptions`](src/download.rs) supporting target video resolutions (`2160p`..`360p`), audio bitrates (`320`..`128` kbps), structured `DownloadProgress` events, and `CancellationToken` aborting.
* **Subscription Interoperability:** Multi-format strategy engine ([`importers.rs`](src/importers.rs)) supporting OPML 2.0 XML, Google Takeout CSV (`subscriptions.csv`), and NewPipe JSON.
* **Resilient API Budgeting:** Local [`QuotaTracker`](src/quota.rs) tracking daily 10,000-unit usage with daily UTC rollover and atomic persistence.
* **Zero-Quota RSS Fallback:** Public Atom/RSS feed parser (`quick-xml`) for browsing channel uploads without API keys or quota consumption.
* **In-Memory & Disk-Backed TTL Caching:** [`TtlCache`](src/cache.rs) and `MetadataCache` providing configurable expiration for video metadata (30m) and channel profiles (2h) across runs.
* **Return YouTube Dislike (RYD):** Non-blocking lookup of restored community dislike counts.
* **In-Memory Mock Double:** Comprehensive [`MockYoutubeClient`](src/testing/mod.rs) allowing full application testing without live credentials or network access.

---

## Cargo Features

| Feature | Default | Description |
| :--- | :---: | :--- |
| `audio` | No | Enables the background audio decoding worker using `rodio`. |
| `download` | No | Enables `yt-dlp` subprocess execution, `DownloadProgress`, and `CancellationToken` tracking. |
| `importers` | No | Enables OPML XML, Google Takeout CSV, and NewPipe JSON subscription importers/exporters. |
| `sponsorblock` | No | Enables SponsorBlock segment fetching and native MPV `edl://` URL generation. |
| `ipc` | No | Enables bi-directional MPV IPC over Windows named pipes and Unix domain sockets. |
| `full` | No | Enables all optional modules (`audio`, `download`, `importers`, `sponsorblock`, `ipc`). |

```toml
[dependencies]
youtube-client-lib = { version = "0.3.0", features = ["full"] }
```

---

## Quickstart

```rust,no_run
use youtube_client_lib::{YoutubeClient, VideoService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = YoutubeClient::builder()
        .with_credentials("MY_CLIENT_ID", "MY_CLIENT_SECRET")
        .with_token_cache("tokencache.json")
        .build()
        .await?;

    let videos = client.list_videos("UC_x5XG1OV2P6uZZ5FSM9Ttw", 10).await?;
    for video in videos {
        println!("{} ({})", video.title, video.id);
    }

    Ok(())
}
```

---

## Documentation

For full architectural details, see the workspace documentation:
* [Architecture Specification](../docs/ARCHITECTURE.md)
* [SOLID Architecture](../docs/solid_ARCHITECTURE.md)
* [Video & Audio Playback Architecture](../docs/VIDEO_PLAYBACK.md)
* [API Quota & Caching Guide](../docs/CACHING_AND_QUOTA.md)
* [Configuration Guide](../docs/CONFIGURATION.md)
