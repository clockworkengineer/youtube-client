use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::Video;

pub async fn execute_videos(
    ctx: &CliContext,
    channel_id: String,
    limit: u32,
    page_token: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    if !json {
        println!("Fetching videos for channel {channel_id} (limit: {limit})...");
    }
    let page = client
        .list_videos_page(&channel_id, limit, page_token.as_deref())
        .await?;

    let formatter = select_formatter::<Video>(json);
    println!("{}", formatter.format_page(&page));

    Ok(())
}
