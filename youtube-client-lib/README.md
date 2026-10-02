# `youtube-client-lib` (v0.2.0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Crate](https://img.shields.io/badge/crates.io-v0.2.0-orange.svg)](https://crates.io/crates/youtube-client-lib)
[![Documentation](https://docs.rs/youtube-client-lib/badge.svg)](https://docs.rs/youtube-client-lib)

A modular, asynchronous Rust library for interacting with the YouTube Data API v3, managing subscriptions, browsing uploaded videos and playlists, streaming audio via Rodio, and downloading media streams via yt-dlp.

---

## Features

* **SOLID Clean Architecture:** Engineered around segregated service traits (`VideoService`, `SubscriptionService`, `PlaylistService`, `CommentService`, `MediaDownloader`) and a unified `YoutubeBackend` adapter.
* **Pluggable Media Players:** Unified [`MediaPlayer`](src/player.rs) trait and auto-detecting [`PlayerRegistry`](src/player.rs) supporting MPV, VLC, IINA, custom binaries, and system defaults.
* **Subscription Interoperability:** Multi-format strategy engine supporting OPML XML, Google Takeout CSV (`subscriptions.csv`), and NewPipe JSON.
* **Resilient API Budgeting:** Local [`QuotaTracker`](src/quota.rs) tracking daily 10,000 unit usage with daily UTC rollover and atomic persistence.
* **Zero-Quota RSS Fallback:** Public Atom/RSS feed parser for browsing channel uploads without API keys or quota consumption.
* **In-Memory TTL Caching:** [`TtlCache`](src/cache.rs) providing configurable expiration for video metadata (30m) and channel profiles (2h).
* **Return YouTube Dislike (RYD):** Non-blocking lookup of restored community dislike counts.
* **In-Memory Mock Double:** Comprehensive [`MockYoutubeClient`](src/testing/mod.rs) allowing full application testing without live credentials or network access.

---

## Cargo Features

| Feature | Default | Description |
| :--- | :---: | :--- |
| `audio` | No | Enables the background audio decoding worker using `rodio`. |
| `download` | No | Enables `yt-dlp` subprocess execution and media download tracking. |

```toml
[dependencies]
youtube-client-lib = { version = "0.2.0", features = ["audio", "download"] }
```

---

## Quickstart

```rust,no_run
use youtube_client_lib::{YoutubeClient, VideoService, Rating};
use std::path::Path;

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
* [Architecture Specification](../docs/architecture.md)
* [SOLID Architecture](../docs/solid_architecture.md)
* [API Quota & Caching Guide](../docs/caching_and_quota.md)
* [Configuration Guide](../docs/configuration.md)
