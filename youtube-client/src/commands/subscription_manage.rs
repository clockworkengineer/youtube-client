use crate::commands::context::CliContext;

pub async fn execute_subscribe(
    ctx: &CliContext,
    channel_id: String,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Subscribing to channel {}...", channel_id);
    client.subscribe_to_channel(&channel_id).await?;
    println!("✓ Successfully subscribed to channel {}!", channel_id);
    Ok(())
}

pub async fn execute_unsubscribe(
    ctx: &CliContext,
    subscription_id: String,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Unsubscribing from subscription {}...", subscription_id);
    client.unsubscribe_from_channel(&subscription_id).await?;
    println!("✓ Successfully unsubscribed from subscription {}!", subscription_id);
    Ok(())
}
