use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::{DurationFilter, SearchFilter, SearchOrder, SearchType, Video};

#[allow(clippy::too_many_arguments)]
pub async fn execute_search(
    ctx: &CliContext,
    query: String,
    limit: u32,
    order: Option<String>,
    duration: Option<String>,
    content_type: Option<String>,
    page_token: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;

    let has_filter = order.is_some() || duration.is_some() || content_type.is_some();
    let filter = if has_filter {
        Some(SearchFilter {
            order: order.as_deref().map(SearchOrder::from_str_loose),
            duration: duration.as_deref().map(DurationFilter::from_str_loose),
            content_type: content_type.as_deref().map(SearchType::from_str_loose),
            published_after: None,
        })
    } else {
        None
    };

    let page = client
        .search_videos_page_with_filter(&query, limit, page_token.as_deref(), filter.as_ref())
        .await?;

    if !json {
        println!("Search results for '{query}' (limit: {limit}):");
    }
    let formatter = select_formatter::<Video>(json);
    println!("{}", formatter.format_page(&page));

    Ok(())
}
