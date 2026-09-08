use crate::commands::context::CliContext;
use youtube_client_lib::models::Rating;

pub async fn execute_rate(
    ctx: &CliContext,
    video_id: String,
    rating: Rating,
) -> anyhow::Result<()> {
    let client = ctx.get_client().await?;
    println!("Submitting rating '{:?}' for video ID {}...", rating, video_id);
    client.rate_video(&video_id, rating).await?;
    println!("Successfully rated video {} as {:?}.", video_id, rating);
    Ok(())
}
