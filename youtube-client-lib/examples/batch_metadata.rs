//! # Example: Batch Video Metadata & Quota Tracking
//!
//! Demonstrates:
//! - Multi-ID batch querying (`get_videos_batch`) chunked in up to 50 videos per API call.
//! - Disk-backed persistent `MetadataCache` to survive process invocations.
//! - `QuotaTracker` for tracking YouTube Data API v3 units consumed.
//!
//! Run with:
//! ```bash
//! cargo run --example batch_metadata
//! ```

use std::sync::Arc;
use youtube_client_lib::{MetadataCache, MockYoutubeClient, QuotaTracker, VideoService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YouTube Client: Batch Metadata & Quota Tracking Example ===");

    // Set up a temporary disk-backed persistent metadata cache
    let temp_cache_file = std::env::temp_dir().join("youtube_example_cache.json");
    let cache = Arc::new(MetadataCache::open(&temp_cache_file));
    println!("Opened disk-backed cache at: {temp_cache_file:?}");

    // Set up a quota budget tracker (standard default: 10,000 units/day)
    let quota = Arc::new(QuotaTracker::new());
    println!("Initial quota consumed: {} units", quota.status().units_used);

    // Use Mock client for offline example execution
    let mock = MockYoutubeClient::new();
    let video_ids = vec!["dQw4w9WgXcQ", "9bZkp7q19f0", "kJQP7kiw5Fk"];

    println!("Querying batch of {} videos...", video_ids.len());
    let details = mock.get_videos_batch(&video_ids).await?;

    for video in &details {
        println!(
            "- Video: {} | Views: {} | Duration: {}",
            video.title, video.view_count, video.duration_formatted
        );
        cache.set_video(&video.id, video.clone());
    }

    println!("Flushed cache to disk. Verifying persistent entries:");
    let reopened = MetadataCache::open(&temp_cache_file);
    for id in &video_ids {
        if let Some(cached) = reopened.get_video(id) {
            println!("  [Persisted] {} -> {}", cached.id, cached.title);
        }
    }

    // Clean up temporary cache file
    let _ = std::fs::remove_file(temp_cache_file);
    println!("Batch metadata and cache demonstration finished successfully.");
    Ok(())
}
