//! # Example: SponsorBlock Integration & MPV EDL Generation
//!
//! Demonstrates:
//! - Querying SponsorBlock skip segments for a video.
//! - Generating MPV EDL (`edl://...`) URLs to skip sponsors automatically without plugins.
//! - MPV IPC named pipe client abstraction.
//!
//! Run with:
//! ```bash
//! cargo run --example sponsorblock_mpv
//! ```

use youtube_client_lib::{
    build_mpv_edl, fetch_skip_segments, MpvIpcClient, SegmentCategory, SkipSegment,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YouTube Client: SponsorBlock & MPV Integration Example ===");

    let http = reqwest::Client::new();
    let video_id = "dQw4w9WgXcQ";

    println!("Querying SponsorBlock segments for {video_id}...");
    match fetch_skip_segments(&http, video_id, None).await {
        Ok(segments) => {
            println!("Retrieved {} skip segments:", segments.len());
            for seg in &segments {
                println!(
                    "  - [{:?}] {:.1}s - {:.1}s (Action: {})",
                    seg.category, seg.start_secs, seg.end_secs, seg.action_type
                );
            }

            // Demonstrate synthetic EDL generation
            let mock_segments = if segments.is_empty() {
                vec![SkipSegment {
                    category: SegmentCategory::Sponsor,
                    action_type: "skip".to_string(),
                    start_secs: 30.0,
                    end_secs: 45.0,
                    uuid: "demo_uuid".to_string(),
                }]
            } else {
                segments
            };

            let video_url = format!("https://www.youtube.com/watch?v={video_id}");
            let total_duration_secs = 212.0;

            if let Some(edl_url) = build_mpv_edl(&video_url, total_duration_secs, &mock_segments) {
                println!("\nGenerated MPV EDL Stream URL:\n{edl_url}");
                println!("Passing this EDL URL to MPV automatically cuts out sponsor segments during playback!");
            }
        }
        Err(e) => {
            println!("SponsorBlock query returned: {e}");
        }
    }

    // Demonstrate MPV IPC pipe path generation
    let ipc_pipe = MpvIpcClient::generate_pipe_path("example-instance");
    println!("\nPlatform MPV IPC Pipe Path: {ipc_pipe}");
    let _ipc_client = MpvIpcClient::new(&ipc_pipe);

    Ok(())
}
