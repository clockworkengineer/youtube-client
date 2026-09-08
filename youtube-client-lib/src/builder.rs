//! # Fluent Builder for [`YoutubeClient`]

use std::path::{Path, PathBuf};
use crate::auth::delegate::OpenBrowserFlowDelegate;
use crate::client::{YoutubeClient, YOUTUBE_SCOPES};
use crate::error::Result;
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;
use yup_oauth2::InstalledFlowReturnMethod;

/// A fluent builder for configuring and constructing a [`YoutubeClient`].
pub struct YoutubeClientBuilder {
    client_id: Option<String>,
    client_secret: Option<String>,
    token_cache_path: PathBuf,
    scopes: Vec<String>,
    return_method: InstalledFlowReturnMethod,
    flow_delegate: Option<Box<dyn InstalledFlowDelegate>>,
}

impl Default for YoutubeClientBuilder {
    fn default() -> Self {
        Self {
            client_id: None,
            client_secret: None,
            token_cache_path: PathBuf::from("tokencache.json"),
            scopes: YOUTUBE_SCOPES.iter().map(|s| s.to_string()).collect(),
            return_method: InstalledFlowReturnMethod::HTTPRedirect,
            flow_delegate: None,
        }
    }
}

impl YoutubeClientBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Google OAuth2 Client ID and Client Secret.
    pub fn with_credentials(mut self, client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Set the path to the disk token cache file.
    pub fn with_token_cache(mut self, path: impl AsRef<Path>) -> Self {
        self.token_cache_path = path.as_ref().to_path_buf();
        self
    }

    /// Set the OAuth scopes required by the client.
    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the installed flow return method (e.g. HTTPRedirect or Interactive).
    pub fn with_return_method(mut self, method: InstalledFlowReturnMethod) -> Self {
        self.return_method = method;
        self
    }

    /// Provide a custom flow delegate for handling browser opening or URL presentation.
    pub fn with_flow_delegate(mut self, delegate: Box<dyn InstalledFlowDelegate>) -> Self {
        self.flow_delegate = Some(delegate);
        self
    }

    /// Build and authenticate the [`YoutubeClient`].
    pub async fn build(self) -> Result<YoutubeClient> {
        let client_id = self.client_id.ok_or_else(|| {
            crate::error::YoutubeError::Credentials("Client ID must be specified".to_string())
        })?;
        let client_secret = self.client_secret.ok_or_else(|| {
            crate::error::YoutubeError::Credentials("Client Secret must be specified".to_string())
        })?;

        let delegate = self.flow_delegate.unwrap_or_else(|| Box::new(OpenBrowserFlowDelegate));

        YoutubeClient::construct_with_params(
            &client_id,
            &client_secret,
            &self.token_cache_path,
            &self.scopes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            self.return_method,
            delegate,
        )
        .await
    }
}
