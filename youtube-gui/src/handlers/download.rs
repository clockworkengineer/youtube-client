use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::actions::spawn_download;
use crate::types::{AppState, PendingAction};

/// Handle media download actions (Single Responsibility Principle).
pub fn handle_download_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    ctx: &egui::Context,
) -> bool {
    match action {
        PendingAction::SpawnDownload {
            video,
            is_audio,
            resolution,
            audio_bitrate,
        } => {
            spawn_download(
                state.clone(),
                ctx.clone(),
                video.clone(),
                *is_audio,
                *resolution,
                *audio_bitrate,
            );
            true
        }
        _ => false,
    }
}
