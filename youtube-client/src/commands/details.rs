use crate::commands::context::CliContext;

pub async fn execute_details(ctx: &CliContext, video_id: String, json: bool) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let details = client.fetch_video_details(&video_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&details)?);
        return Ok(());
    }

    println!("==================================================");
    println!("  🎬 {}", details.title);
    println!("==================================================");
    println!("  Video ID:        {}", details.id);
    println!(
        "  Channel:         {} ({})",
        details.channel_title, details.channel_id
    );
    println!("  Published:       {}", details.published_at);
    println!(
        "  Duration:        {} ({} seconds)",
        details.duration_formatted, details.duration_seconds
    );
    println!("  Views:           {}", details.view_count);
    println!("  Likes:           {}", details.like_count);
    println!("  Comments:        {}", details.comment_count);
    if !details.tags.is_empty() {
        println!("  Tags:            {}", details.tags.join(", "));
    }
    println!("--------------------------------------------------");
    println!("Description:\n{}", details.description);
    println!("==================================================");

    Ok(())
}
