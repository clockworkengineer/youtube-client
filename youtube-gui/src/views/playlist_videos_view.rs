use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::Video;

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::draw_video_card;

pub fn render_playlist_videos_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlist_id: &str,
    playlist_title: &str,
    videos: &Option<Result<Vec<Video>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        if ui.button("⬅ Back to Playlists").clicked() {
            action = Some(PendingAction::LoadPlaylists);
        }
        ui.heading(format!("📂 Playlist: {playlist_title}"));
    });
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
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Failed to load playlist videos",
                );
                ui.add_space(10.0);
                ui.label(err_msg);
                ui.add_space(20.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetryPlaylistVideos {
                        id: playlist_id.to_string(),
                        title: playlist_title.to_string(),
                    });
                }
            });
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
                        for video in vids {
                            let mut card_action = PendingAction::None;
                            draw_video_card(
                                state,
                                http_client,
                                audio_tx,
                                ui,
                                ctx,
                                video,
                                &mut card_action,
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
