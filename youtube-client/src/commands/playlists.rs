use crate::commands::context::CliContext;
use youtube_client_lib::utils::{print_table, truncate};

pub async fn execute_playlists(
    ctx: &CliContext,
    limit: u32,
    playlist_id: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;

    if let Some(pid) = playlist_id {
        println!("Fetching videos for playlist {pid} (limit: {limit})...");
        let videos = client.list_playlist_videos(&pid, limit).await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&videos)?);
            return Ok(());
        }

        print_table(
            &["Index", "Title", "Video ID", "Published At"],
            &[5, 40, 15, 15],
            &videos,
            |vid, idx| {
                vec![
                    (idx + 1).to_string(),
                    truncate(&vid.title, 38).into_owned(),
                    vid.id.clone(),
                    truncate(&vid.published_at, 10).into_owned(),
                ]
            },
        );
    } else {
        println!("Fetching playlists (limit: {limit})...");
        let playlists = client.list_playlists(limit).await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&playlists)?);
            return Ok(());
        }

        print_table(
            &["Index", "Title", "Playlist ID", "Item Count"],
            &[5, 40, 30, 12],
            &playlists,
            |pl, idx| {
                vec![
                    (idx + 1).to_string(),
                    truncate(&pl.title, 38).into_owned(),
                    pl.id.clone(),
                    pl.video_count.to_string(),
                ]
            },
        );
    }

    Ok(())
}
