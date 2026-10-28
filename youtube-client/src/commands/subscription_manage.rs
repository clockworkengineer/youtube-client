use crate::commands::context::CliContext;
use std::path::PathBuf;
use youtube_client_lib::{
    NewPipeJsonFormat, OpmlFormat, SubscriptionFormat, SubscriptionFormatRegistry,
    SubscriptionImport, TakeoutCsvFormat,
};

pub async fn execute_subscribe(ctx: &CliContext, channel_id: String) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Subscribing to channel {channel_id}...");
    client.subscribe_to_channel(&channel_id).await?;
    println!("✓ Successfully subscribed to channel {channel_id}!");
    Ok(())
}

pub async fn execute_unsubscribe(ctx: &CliContext, subscription_id: String) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Unsubscribing from subscription {subscription_id}...");
    client.unsubscribe_from_channel(&subscription_id).await?;
    println!("✓ Successfully unsubscribed from subscription {subscription_id}!");
    Ok(())
}

pub async fn execute_export_subscriptions(
    ctx: &CliContext,
    format: String,
    output: PathBuf,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Fetching all subscriptions for export...");

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

    let imports: Vec<SubscriptionImport> = all_subs
        .iter()
        .map(SubscriptionImport::from)
        .collect();

    let fmt_lower = format.trim().to_ascii_lowercase();
    let serialized = match fmt_lower.as_str() {
        "opml" | "xml" => OpmlFormat
            .serialize(&imports)
            .map_err(|e| anyhow::anyhow!(e))?,
        "csv" | "takeout" | "takeout_csv" => TakeoutCsvFormat
            .serialize(&imports)
            .map_err(|e| anyhow::anyhow!(e))?,
        "json" | "newpipe" | "newpipe_json" => NewPipeJsonFormat
            .serialize(&imports)
            .map_err(|e| anyhow::anyhow!(e))?,
        other => anyhow::bail!(
            "Unsupported export format '{other}'. Use 'opml', 'csv', or 'json'."
        ),
    };

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&output, serialized)?;
    println!(
        "✓ Exported {} subscriptions ({}) to {}",
        imports.len(),
        fmt_lower,
        output.display()
    );
    Ok(())
}

pub async fn execute_import_subscriptions(
    ctx: &CliContext,
    format: String,
    file: PathBuf,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {e}", file.display()))?;

    let fmt_lower = format.trim().to_ascii_lowercase();
    let imports = match fmt_lower.as_str() {
        "auto" => SubscriptionFormatRegistry::with_defaults()
            .import_auto_detect(&content, Some(&file))
            .map_err(|e| anyhow::anyhow!(e))?,
        "opml" | "xml" => OpmlFormat
            .parse(&content)
            .map_err(|e| anyhow::anyhow!(e))?,
        "csv" | "takeout" | "takeout_csv" => TakeoutCsvFormat
            .parse(&content)
            .map_err(|e| anyhow::anyhow!(e))?,
        "json" | "newpipe" | "newpipe_json" => NewPipeJsonFormat
            .parse(&content)
            .map_err(|e| anyhow::anyhow!(e))?,
        other => anyhow::bail!(
            "Unsupported import format '{other}'. Use 'auto', 'opml', 'csv', or 'json'."
        ),
    };

    println!(
        "Parsed {} channels from {}. Subscribing...",
        imports.len(),
        file.display()
    );

    let client = ctx.get_client().await?;
    let mut success_count = 0usize;
    for item in &imports {
        match client.subscribe_to_channel(&item.channel_id).await {
            Ok(()) => {
                success_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "⚠ Failed to subscribe to '{}' ({}): {e}",
                    item.channel_title, item.channel_id
                );
            }
        }
    }

    println!(
        "✓ Successfully imported {success_count}/{} subscriptions!",
        imports.len()
    );
    Ok(())
}
