use crate::commands::context::CliContext;
use youtube_client_lib::models::Subscription;
use youtube_client_lib::utils::{print_table, truncate};

pub async fn execute_subscriptions(
    ctx: &CliContext,
    limit: u32,
    page_token: Option<String>,
    all: bool,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;

    if all {
        println!("Fetching all subscriptions (paginated)...");
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

        if json {
            println!("{}", serde_json::to_string_pretty(&all_subs)?);
            return Ok(());
        }

        print_subscriptions_table(&all_subs);
        println!("\nTotal subscriptions: {}", all_subs.len());
        return Ok(());
    }

    let page = client
        .list_subscriptions_page(limit, page_token.as_deref())
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
        return Ok(());
    }

    println!("Subscriptions (limit: {limit}):");
    print_subscriptions_table(&page.items);

    if let Some(next) = &page.next_page_token {
        println!("\nNext page token: {next}");
        println!("Fetch next page with: --page-token {next}");
    }

    Ok(())
}

fn print_subscriptions_table(subs: &[Subscription]) {
    print_table(
        &["Index", "Title", "Channel ID"],
        &[5, 35, 30],
        subs,
        |sub, idx| {
            vec![
                (idx + 1).to_string(),
                truncate(&sub.title, 33).into_owned(),
                sub.channel_id.clone(),
            ]
        },
    );
}
