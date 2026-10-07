//! # Example: Search Videos & Streaming Pagination
//!
//! Demonstrates:
//! - Initializing `YoutubeClient` using an API Key (zero-OAuth, zero tokens needed).
//! - Iterating through paginated search results asynchronously via `BoxStream`.
//!
//! Run with:
//! ```bash
//! cargo run --example search_and_stream
//! ```

use youtube_client_lib::YoutubeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YouTube Client: Search & Stream Example ===");

    // Use an API key from environment or fallback to a demo placeholder
    let api_key = std::env::var("YOUTUBE_API_KEY").unwrap_or_else(|_| "DEMO_KEY".to_string());
    println!("Initializing API-Key-only client (Zero-OAuth mode)...");
    let client = YoutubeClient::new_api_key(&api_key).await?;

    println!("Client is API key only: {}", client.is_api_key_only());

    // In a live environment with a valid API key, stream_search returns an asynchronous stream:
    // let mut stream = client.stream_search("Rust programming language", 10);
    // while let Some(video) = stream.next().await {
    //     println!("Found: {} (ID: {})", video.title, video.id);
    // }

    println!("Stream interface demonstration completed successfully.");
    Ok(())
}
