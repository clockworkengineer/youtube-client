use crate::commands::context::CliContext;

pub async fn execute_login(ctx: &CliContext) -> anyhow::Result<()> {
    println!("Starting OAuth2 Login flow with full YouTube permissions...");
    let _client = ctx.get_client().await?;
    println!("✓ Login successful! Token saved to {:?}", ctx.token_cache);
    Ok(())
}
