use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::models::Comment;

pub async fn execute_comments(
    ctx: &CliContext,
    video_id: String,
    _limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    let comments = client.fetch_comments(&video_id).await?;

    if !json {
        if comments.is_empty() {
            println!("No comments found for video {video_id}.");
            return Ok(());
        }
        println!(
            "Comments for video {} ({} comments):",
            video_id,
            comments.len()
        );
    }

    let formatter = select_formatter::<Comment>(json);
    println!("{}", formatter.format_list(&comments));

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
