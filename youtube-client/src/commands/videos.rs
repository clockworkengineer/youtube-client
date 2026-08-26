use std::path::PathBuf;
use youtube_client_lib::utils::{print_table, truncate};
use youtube_client_lib::init_client;

pub async fn execute_videos(
    client_id: Option<String>,
    client_secret: Option<String>,
    config: &PathBuf,
    token_cache: &PathBuf,
    channel_id: String,
    limit: u32,
) -> anyhow::Result<()> {
    let client = init_client(client_id, client_secret, config, token_cache).await?;
    println!("Fetching videos for channel {}...", channel_id);
    let videos = client.list_videos(&channel_id, limit).await?;
    print_table(
        &["Index", "Title", "Video ID", "Published At"],
        &[5, 40, 15, 15],
        &videos,
        |vid, idx| vec![
            (idx + 1).to_string(),
            truncate(&vid.title, 38).into_owned(),
            vid.id.clone(),
            truncate(&vid.published_at, 10).into_owned(),
        ],
    );
    Ok(())
}
