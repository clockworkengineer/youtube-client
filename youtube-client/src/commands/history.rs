use std::path::PathBuf;
use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use youtube_client_lib::utils::{
    clear_watch_history, get_watch_history, remove_watch_history_entry, HistoryEntry,
};

pub async fn execute_history(
    _ctx: &CliContext,
    clear: bool,
    remove: Option<String>,
    limit: Option<usize>,
    file: Option<PathBuf>,
    json: bool,
) -> anyhow::Result<()> {
    let history_file = file.unwrap_or_else(|| {
        youtube_client_lib::resolve_app_data_path("playback_positions.json")
    });

    if clear {
        clear_watch_history(&history_file)
            .map_err(|e| anyhow::anyhow!("Failed to clear watch history: {e}"))?;
        println!("✓ Successfully cleared watch history.");
        return Ok(());
    }

    if let Some(video_id) = remove {
        remove_watch_history_entry(&history_file, &video_id)
            .map_err(|e| anyhow::anyhow!("Failed to remove video {video_id} from history: {e}"))?;
        println!("✓ Successfully removed '{video_id}' from watch history.");
        return Ok(());
    }

    let mut entries = get_watch_history(&history_file);
    if let Some(n) = limit {
        entries.truncate(n);
    }

    let formatter = select_formatter::<HistoryEntry>(json);
    println!("{}", formatter.format_list(&entries));

    Ok(())
}
