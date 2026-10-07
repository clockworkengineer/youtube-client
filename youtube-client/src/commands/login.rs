use crate::commands::context::CliContext;

pub async fn execute_login(ctx: &CliContext, device_code: bool) -> anyhow::Result<()> {
    if device_code {
        println!("Starting OAuth2 Interactive / Device terminal authentication flow...");
        let _client = ctx.get_client_interactive().await?;
    } else {
        println!("Starting OAuth2 Login flow with full YouTube permissions...");
        println!("A browser window will open shortly. Please sign in and allow access...");
        let _client = ctx.get_client().await?;
    }
    println!("✓ Login successful! Token saved to {:?}", ctx.token_cache);
    Ok(())
}
