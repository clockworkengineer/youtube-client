use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::Video;

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::{draw_video_card_in_playlist, render_error_banner};

#[allow(clippy::too_many_arguments)]
pub fn render_playlist_videos_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlist_id: &str,
    playlist_title: &str,
    videos: &Option<Result<Arc<Vec<Video>>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        if ui
            .button("⬅ Back")
            .on_hover_text("Return to Playlists (Esc / Alt+Left)")
            .clicked()
        {
            action = Some(PendingAction::GoBack);
        }
        ui.add_space(10.0);
        ui.heading(
            egui::RichText::new(format!("📂 {playlist_title}"))
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        )
        .on_hover_text(format!("Playlist ID: {playlist_id}"));
        if let Some(Ok(vids)) = videos {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("({} videos)", vids.len()))
                    .size(13.0)
                    .color(egui::Color32::GRAY),
            );
        }
        ui.add_space(16.0);
        if ui
            .button("🔄 Refresh")
            .on_hover_text("Reload videos in this playlist (F5)")
            .clicked()
        {
            action = Some(PendingAction::RetryPlaylistVideos {
                id: playlist_id.to_string(),
                title: playlist_title.to_string(),
            });
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    match videos {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.spinner();
                ui.label("Loading playlist videos...");
            });
        }
        Some(Err(err_msg)) => {
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Failed to load playlist videos",
                err_msg,
                PendingAction::RetryPlaylistVideos {
                    id: playlist_id.to_string(),
                    title: playlist_title.to_string(),
                },
                &mut banner_action,
            );
            if !matches!(banner_action, PendingAction::None) {
                action = Some(banner_action);
            }
        }
        Some(Ok(vids)) => {
            if vids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No videos found in this playlist.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for video in vids.iter() {
                            let mut card_action = PendingAction::None;
                            draw_video_card_in_playlist(
                                state,
                                http_client,
                                audio_tx,
                                ui,
                                ctx,
                                video,
                                &mut card_action,
                                false,
                                Some((playlist_id, playlist_title)),
                            );
                            if !matches!(card_action, PendingAction::None) {
                                action = Some(card_action);
                            }
                        }
                    });
            }
        }
    }

    action
}
