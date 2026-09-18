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
