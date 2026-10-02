use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::Video;

pub async fn execute_search(
    ctx: &CliContext,
    query: String,
    limit: u32,
    page_token: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let page = client
        .search_videos_page(&query, limit, page_token.as_deref())
        .await?;

    if !json {
        println!("Search results for '{query}' (limit: {limit}):");
    }
    let formatter = select_formatter::<Video>(json);
    println!("{}", formatter.format_page(&page));

    Ok(())
}
