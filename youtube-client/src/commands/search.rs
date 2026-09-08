use crate::commands::context::CliContext;
use youtube_client_lib::utils::{print_table, truncate};

pub async fn execute_search(
    ctx: &CliContext,
    query: String,
    limit: u32,
    page_token: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let page = client.search_videos_page(&query, limit, page_token.as_deref()).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
        return Ok(());
    }

    println!("Search results for '{}' (limit: {}):", query, limit);
    print_table(
        &["Index", "Title", "Video ID", "Published At"],
        &[5, 40, 15, 15],
        &page.items,
        |vid, idx| vec![
            (idx + 1).to_string(),
            truncate(&vid.title, 38).into_owned(),
            vid.id.clone(),
            truncate(&vid.published_at, 10).into_owned(),
        ],
    );

    if let Some(next) = &page.next_page_token {
        println!("\nNext page token: {}", next);
        println!("Fetch next page with: --page-token {}", next);
    }

    Ok(())
}
