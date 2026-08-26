use std::path::PathBuf;
use youtube_client_lib::utils::{print_table, truncate};
use youtube_client_lib::init_client;

pub async fn execute_subscriptions(
    client_id: Option<String>,
    client_secret: Option<String>,
    config: &PathBuf,
    token_cache: &PathBuf,
    limit: u32,
) -> anyhow::Result<()> {
    let client = init_client(client_id, client_secret, config, token_cache).await?;
    println!("Fetching subscriptions...");
    let subs = client.list_subscriptions(limit).await?;
    print_table(
        &["Index", "Title", "Channel ID"],
        &[5, 30, 30],
        &subs,
        |sub, idx| vec![(idx + 1).to_string(), truncate(&sub.title, 28).into_owned(), sub.channel_id.clone()],
    );
    Ok(())
}
