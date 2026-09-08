use std::path::PathBuf;
use youtube_client_lib::{YoutubeClient, YoutubeClientBuilder};

/// Execution context passed to CLI subcommands.
#[derive(Clone, Debug)]
pub struct CliContext {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub config: PathBuf,
    pub token_cache: PathBuf,
}

impl CliContext {
    /// Instantiate a configured `YoutubeClient` using the fluent builder.
    pub async fn get_client(&self) -> anyhow::Result<YoutubeClient> {
        let mut builder = YoutubeClientBuilder::new()
            .with_config_path(&self.config)
            .with_token_cache(&self.token_cache);

        if let (Some(id), Some(secret)) = (&self.client_id, &self.client_secret) {
            builder = builder.with_credentials(id, secret);
        }

        let client = builder.build().await?;
        Ok(client)
    }
}
