use std::path::PathBuf;
use youtube_client_lib::{
    InteractiveFlowDelegate, MockYoutubeClient, YoutubeBackend, YoutubeClientBuilder,
};

/// Execution context passed to CLI subcommands.
#[derive(Clone, Debug, Default)]
pub struct CliContext {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub api_key: Option<String>,
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
            .with_persistent_cache(None);

        if let Some(ref key) = self.api_key {
            builder = builder.with_api_key(key);
        }

        if let (Some(id), Some(secret)) = (&self.client_id, &self.client_secret) {
            builder = builder.with_credentials(id, secret);
        }

        let client = builder.build().await?;
        Ok(YoutubeBackend::live(client))
    }

    /// Obtain a live `YoutubeBackend` using terminal interactive flow (prompts code for headless/SSH).
    pub async fn get_client_interactive(&self) -> anyhow::Result<YoutubeBackend> {
        if let Some(ref mock) = self.mock_client {
            return Ok(YoutubeBackend::mock(mock.clone()));
        }

        let mut builder = YoutubeClientBuilder::new()
            .with_config_path(&self.config)
            .with_token_cache(&self.token_cache)
            .with_persistent_cache(None)
            .with_return_method(youtube_client_lib::yup_oauth2::InstalledFlowReturnMethod::Interactive)
            .with_flow_delegate(Box::new(InteractiveFlowDelegate));

        if let Some(ref key) = self.api_key {
            builder = builder.with_api_key(key);
        }

        if let (Some(id), Some(secret)) = (&self.client_id, &self.client_secret) {
            builder = builder.with_credentials(id, secret);
        }

        let client = builder.build().await?;
        Ok(YoutubeBackend::live(client))
    }
}
