use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::VideoDetails;

pub async fn execute_details(ctx: &CliContext, video_id: String, json: bool) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let details = client.fetch_video_details(&video_id).await?;

    let formatter = select_formatter::<VideoDetails>(json);
    println!("{}", formatter.format_item(&details));

    Ok(())
}
