use crate::commands::context::CliContext;

pub async fn execute_playlist_create(
    ctx: &CliContext,
    title: String,
    description: Option<String>,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Creating playlist '{title}'...");
    let playlist = client
        .create_playlist(&title, description.as_deref())
        .await?;
    println!("✓ Successfully created playlist!");
    println!("  Title:        {}", playlist.title);
    println!("  Playlist ID:  {}", playlist.id);
    Ok(())
}

pub async fn execute_playlist_delete(ctx: &CliContext, playlist_id: String) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Deleting playlist {playlist_id}...");
    client.delete_playlist(&playlist_id).await?;
    println!("✓ Successfully deleted playlist {playlist_id}!");
    Ok(())
}

pub async fn execute_playlist_remove_video(
    ctx: &CliContext,
    playlist_item_id: String,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Removing item {playlist_item_id} from playlist...");
    client.remove_from_playlist(&playlist_item_id).await?;
    println!("✓ Successfully removed item {playlist_item_id} from playlist!");
    Ok(())
}

pub async fn execute_playlist_update(
    ctx: &CliContext,
    playlist_id: String,
    title: String,
    description: Option<String>,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Updating playlist {playlist_id}...");
    let updated = client
        .update_playlist(&playlist_id, &title, description.as_deref())
        .await?;
    println!("✓ Successfully updated playlist!");
    println!("  Title:       {}", updated.title);
    println!("  Description: {}", updated.description);
    Ok(())
}
