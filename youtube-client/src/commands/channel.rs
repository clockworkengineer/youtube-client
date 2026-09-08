use crate::commands::context::CliContext;

pub async fn execute_channel(
    ctx: &CliContext,
    channel_id: String,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let details = client.get_channel_details(&channel_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&details)?);
        return Ok(());
    }

    println!("==================================================");
    println!("  📺 {}", details.title);
    if let Some(handle) = &details.custom_url {
        println!("  Handle:          {}", handle);
    }
    println!("==================================================");
    println!("  Channel ID:      {}", details.id);
    println!("  Subscribers:     {}", details.subscriber_count);
    println!("  Total Videos:    {}", details.video_count);
    println!("  Lifetime Views:  {}", details.view_count);
    if !details.description.is_empty() {
        println!("--------------------------------------------------");
        println!("About:\n{}", details.description);
    }
    println!("==================================================");

    Ok(())
}
