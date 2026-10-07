#[derive(Copy, Clone, Debug, Default)]
pub struct OpenBrowserFlowDelegate;

impl yup_oauth2::authenticator_delegate::InstalledFlowDelegate for OpenBrowserFlowDelegate {
    fn present_user_url(
        &self,
        url: &str,
        _need_code: bool,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<String, String>> + Send>,
    > {
        let url_str = url.to_string();
        Box::pin(async move {
            println!("Opening browser for OAuth authentication: {url_str}");
            if let Err(e) = open::that(&url_str) {
                eprintln!("Failed to open browser automatically: {e}");
            }
            Ok(String::new())
        })
    }
}

/// Flow delegate that outputs URL and prompts for authorization code in terminal stdin.
/// Ideal for remote SSH sessions and headless servers.
#[derive(Copy, Clone, Debug, Default)]
pub struct InteractiveFlowDelegate;

impl yup_oauth2::authenticator_delegate::InstalledFlowDelegate for InteractiveFlowDelegate {
    fn present_user_url(
        &self,
        url: &str,
        _need_code: bool,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<String, String>> + Send>,
    > {
        let url_str = url.to_string();
        Box::pin(async move {
            use std::io::{self, Write};
            println!("\n========================================================");
            println!("Please visit the following URL to authorize this client:");
            println!("  {url_str}");
            println!("========================================================\n");
            print!("Enter authorization code: ");
            let _ = io::stdout().flush();
            let mut code = String::new();
            io::stdin()
                .read_line(&mut code)
                .map_err(|e| format!("Failed to read authorization code: {e}"))?;
            Ok(code.trim().to_string())
        })
    }
}

/// Delegate for OAuth 2.0 Device Authorization flow (`https://www.google.com/device`).
#[derive(Copy, Clone, Debug, Default)]
pub struct TerminalDeviceFlowDelegate;

impl yup_oauth2::authenticator_delegate::DeviceFlowDelegate for TerminalDeviceFlowDelegate {
    fn present_user_code<'a>(
        &'a self,
        pi: &'a yup_oauth2::authenticator_delegate::DeviceAuthResponse,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        let uri = pi.verification_uri.clone();
        let code = pi.user_code.clone();
        Box::pin(async move {
            println!("\n========================================================");
            println!("To authorize this application on a headless device:");
            println!("  1. Visit: {uri}");
            println!("  2. Enter code: {code}");
            println!("========================================================\n");
        })
    }
}
