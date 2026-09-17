use crate::commands::context::CliContext;

pub async fn execute_login(ctx: &CliContext) -> anyhow::Result<()> {
    println!("Starting OAuth2 Login flow with full YouTube permissions...");
    println!("A browser window will open shortly. Please sign in and allow access...");
    let _client = ctx.get_client().await?;
    println!("✓ Login successful! Token saved to {:?}", ctx.token_cache);
    Ok(())
}
