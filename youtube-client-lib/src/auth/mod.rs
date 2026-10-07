pub mod delegate;

pub use delegate::{InteractiveFlowDelegate, OpenBrowserFlowDelegate, TerminalDeviceFlowDelegate};

/// An unauthenticated token provider that produces `Ok(None)`.
/// Used when the client connects using an API Key instead of an OAuth token.
#[derive(Copy, Clone, Debug, Default)]
pub struct NoAuth;

impl google_youtube3::common::GetToken for NoAuth {
    fn get_token(
        &self,
        _scopes: &[&str],
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = std::result::Result<
                        Option<String>,
                        Box<dyn std::error::Error + Send + Sync + 'static>,
                    >,
                > + Send,
        >,
    > {
        Box::pin(async { Ok(None) })
    }
}
