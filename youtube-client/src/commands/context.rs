use std::path::PathBuf;
use youtube_client_lib::{MockYoutubeClient, YoutubeBackend, YoutubeClientBuilder};

/// Execution context passed to CLI subcommands.
#[derive(Clone, Debug, Default)]
pub struct CliContext {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub config: PathBuf,
    pub token_cache: PathBuf,
    pub log_file: Option<PathBuf>,
    pub cookies_file: Option<PathBuf>,
    pub cookies_from_browser: Option<String>,
    pub mock_client: Option<MockYoutubeClient>,
}

impl CliContext {
    /// Create a test context with an in-memory mock client.
    pub fn with_mock(mock: MockYoutubeClient) -> Self {
        Self {
            mock_client: Some(mock),
            ..Default::default()
        }
    }

    /// Obtain an interchangeable `YoutubeBackend` (either Live Google API client or Mock).
    pub async fn get_client(&self) -> anyhow::Result<YoutubeBackend> {
        if let Some(ref mock) = self.mock_client {
            return Ok(YoutubeBackend::mock(mock.clone()));
        }

        let mut builder = YoutubeClientBuilder::new()
            .with_config_path(&self.config)
            .with_token_cache(&self.token_cache)
            .with_default_cache();

        if let (Some(id), Some(secret)) = (&self.client_id, &self.client_secret) {
            builder = builder.with_credentials(id, secret);
        }

        let client = builder.build().await?;
        Ok(YoutubeBackend::live(client))
    }
}
