use crate::commands::context::CliContext;
use crate::formatters::select_formatter;
use std::path::PathBuf;
use youtube_client_lib::utils::{DownloadedItem, scan_downloaded_items};

pub async fn execute_library(
    _ctx: &CliContext,
    dir: Option<PathBuf>,
    audio_only: bool,
    video_only: bool,
    json: bool,
) -> anyhow::Result<()> {
    let base_dir = if let Some(d) = dir {
        d
    } else {
        let config = youtube_client_lib::load_config();
        config
            .downloads_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("downloads"))
    };

    let mut items = scan_downloaded_items(&base_dir);

    if audio_only {
        items.retain(|i| i.is_audio);
    } else if video_only {
        items.retain(|i| !i.is_audio);
    }

    let formatter = select_formatter::<DownloadedItem>(json);
    println!("{}", formatter.format_list(&items));

    Ok(())
}
