use crate::commands::context::CliContext;

pub async fn execute_playlist_create(
    ctx: &CliContext,
    title: String,
    description: Option<String>,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Creating playlist '{}'...", title);
    let playlist = client.create_playlist(&title, description.as_deref()).await?;
    println!("✓ Successfully created playlist!");
    println!("  Title:        {}", playlist.title);
    println!("  Playlist ID:  {}", playlist.id);
    Ok(())
}

pub async fn execute_playlist_delete(
    ctx: &CliContext,
    playlist_id: String,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Deleting playlist {}...", playlist_id);
    client.delete_playlist(&playlist_id).await?;
    println!("✓ Successfully deleted playlist {}!", playlist_id);
    Ok(())
}
