use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::ChannelDetails;

pub async fn execute_channel(
    ctx: &CliContext,
    channel_id: String,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let details = client.get_channel_details(&channel_id).await?;

    let formatter = select_formatter::<ChannelDetails>(json);
    println!("{}", formatter.format_item(&details));

    Ok(())
}
