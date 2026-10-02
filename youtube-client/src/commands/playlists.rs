use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::{Playlist, Video};

pub async fn execute_playlists(
    ctx: &CliContext,
    limit: u32,
    playlist_id: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;

    if let Some(pid) = playlist_id {
        if !json {
            println!("Fetching videos for playlist {pid} (limit: {limit})...");
        }
        let videos = client.list_playlist_videos(&pid, limit).await?;
        let formatter = select_formatter::<Video>(json);
        println!("{}", formatter.format_list(&videos));
    } else {
        if !json {
            println!("Fetching playlists (limit: {limit})...");
        }
        let playlists = client.list_playlists(limit).await?;
        let formatter = select_formatter::<Playlist>(json);
        println!("{}", formatter.format_list(&playlists));
    }

    Ok(())
}
