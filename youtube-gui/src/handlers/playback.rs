use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::launch_external_player_with_options;
use youtube_client_lib::utils::append_to_log;

use crate::types::{AppState, PendingAction, PlayerCommand, lock_state};

/// Handle media playback, streaming, and player launching actions (Single Responsibility Principle).
pub fn handle_playback_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    audio_tx: &Sender<PlayerCommand>,
) -> bool {
    match action {
        PendingAction::PlayLocal {
            path,
            title,
            video_id,
            start_secs,
        } => {
            let is_mp3 = path.extension().map(|e| e == "mp3").unwrap_or(false);
            let log_file = state.lock().unwrap().log_file.clone();
            let vid = video_id
                .clone()
                .or_else(|| youtube_client_lib::utils::extract_video_id_from_path(path));

            if is_mp3 {
                append_to_log(&log_file, "INFO", &format!("Playing local audio: {path:?}"));
                let _ = audio_tx.send(PlayerCommand::Play {
                    path: path.clone(),
                    title: title.clone(),
                    video_id: vid,
                    start_secs: *start_secs,
                });
            } else {
                append_to_log(&log_file, "INFO", &format!("Opening local video: {path:?}"));
                if let Err(e) = launch_external_player_with_options(
                    path.as_os_str(),
                    Some(&log_file),
                    *start_secs,
                ) {
                    append_to_log(
                        &log_file,
                        "WARN",
                        &format!("{e} Falling back to default file opener."),
                    );
                    let _ = open::that(path);
                }
            }
            true
        }
        PendingAction::StreamVideo {
            video_id,
            start_secs,
        } => {
            let url = if let Some(start) = start_secs {
                if *start > 1.0 {
                    format!("https://www.youtube.com/watch?v={video_id}&t={start:.0}s")
                } else {
                    format!("https://www.youtube.com/watch?v={video_id}")
                }
            } else {
                format!("https://www.youtube.com/watch?v={video_id}")
            };
            let log_file = state.lock().unwrap().log_file.clone();
            append_to_log(&log_file, "INFO", &format!("Video clicked: {url}"));

            if let Err(e) = launch_external_player_with_options(
                std::ffi::OsStr::new(&url),
                Some(&log_file),
                *start_secs,
            ) {
                append_to_log(
                    &log_file,
                    "WARN",
                    &format!("{e} Falling back to default browser."),
                );
                let mut s = lock_state(state);
                s.set_toast("Media player not found; opening in browser", true);
                let _ = open::that(url);
            } else {
                let mut s = lock_state(state);
                s.set_toast("Opening stream in external media player...", false);
            }
            true
        }
        PendingAction::OpenInBrowser { url } => {
            let log_file = state.lock().unwrap().log_file.clone();
            append_to_log(&log_file, "INFO", &format!("Opening in web browser: {url}"));
            let mut s = lock_state(state);
            s.set_toast("Opening in web browser...", false);
            let _ = open::that(url);
            true
        }
        _ => false,
    }
}
