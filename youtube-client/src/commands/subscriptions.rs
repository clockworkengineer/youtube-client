use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::Subscription;

pub async fn execute_subscriptions(
    ctx: &CliContext,
    limit: u32,
    page_token: Option<String>,
    all: bool,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let formatter = select_formatter::<Subscription>(json);

    if all {
        if !json {
            println!("Fetching all subscriptions (paginated)...");
        }
        let mut all_subs = Vec::new();
        let mut next_token = None;

        loop {
            let page = client
                .list_subscriptions_page(50, next_token.as_deref())
                .await?;
            all_subs.extend(page.items);
            if let Some(token) = page.next_page_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        println!("{}", formatter.format_list(&all_subs));
        if !json {
            println!("\nTotal subscriptions: {}", all_subs.len());
        }
        return Ok(());
    }

    let page = client
        .list_subscriptions_page(limit, page_token.as_deref())
        .await?;

    if !json {
        println!("Subscriptions (limit: {limit}):");
    }
    println!("{}", formatter.format_page(&page));

    Ok(())
}
