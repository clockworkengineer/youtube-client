use crate::commands::context::CliContext;
use youtube_client_lib::utils::{print_table, truncate};

pub async fn execute_comments(
    ctx: &CliContext,
    video_id: String,
    _limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let comments = client.fetch_comments(&video_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&comments)?);
        return Ok(());
    }

    if comments.is_empty() {
        println!("No comments found for video {video_id}.");
        return Ok(());
    }

    println!(
        "Comments for video {} ({} comments):",
        video_id,
        comments.len()
    );
    print_table(
        &["Author", "Comment", "Likes", "Published At"],
        &[20, 50, 8, 12],
        &comments,
        |c, _idx| {
            vec![
                truncate(&c.author_name, 18).into_owned(),
                truncate(&c.text_display.replace('\n', " "), 48).into_owned(),
                c.like_count.to_string(),
                truncate(&c.published_at, 10).into_owned(),
            ]
        },
    );

    Ok(())
}

pub async fn execute_comment_post(
    ctx: &CliContext,
    video_id: String,
    text: String,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Posting comment to video {video_id}...");
    let comment = client.post_comment(&video_id, &text).await?;
    println!(
        "✓ Successfully posted comment as {}: \"{}\"",
        comment.author_name, comment.text_display
    );
    Ok(())
}
