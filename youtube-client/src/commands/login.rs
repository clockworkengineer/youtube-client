use std::path::PathBuf;
use youtube_client_lib::init_client;

pub async fn execute_login(
    client_id: Option<String>,
    client_secret: Option<String>,
    config: &PathBuf,
    token_cache: &PathBuf,
) -> anyhow::Result<()> {
    println!("Starting OAuth2 Login flow with full YouTube permissions...");
    let _client = init_client(client_id, client_secret, config, token_cache).await?;
    println!("Login successful! Token saved to {:?}", token_cache);
    Ok(())
}
