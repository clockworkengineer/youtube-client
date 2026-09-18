use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::Video;

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::draw_video_card_with_dismiss;

pub fn render_new_videos_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    new_videos: &Option<Result<Vec<Video>, String>>,
    cleared_video_count: usize,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("🆕 New Videos Feed")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(20.0);
        if ui.button("🔄 Refresh").clicked() {
            action = Some(PendingAction::LoadNewVideos);
        }
        ui.add_space(10.0);
        if ui
            .button("🗑️ Clear All")
            .on_hover_text("Clear all videos from New Videos feed")
            .clicked()
        {
            action = Some(PendingAction::ClearAllNewVideos);
        }
        if cleared_video_count > 0 {
            ui.add_space(10.0);
            if ui
                .button(format!("↺ Reset Cleared ({cleared_video_count})"))
                .on_hover_text("Restore all cleared/dismissed videos")
                .clicked()
            {
                action = Some(PendingAction::ResetClearedVideos);
            }
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    match new_videos {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Loading latest videos from your subscriptions...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Failed to load new videos",
                );
                ui.add_space(10.0);
                ui.label(err_msg);
                ui.add_space(20.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::LoadNewVideos);
                }
            });
        }
        Some(Ok(vids)) => {
            if vids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No new videos available.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for video in vids {
                            let mut card_action = PendingAction::None;
                            draw_video_card_with_dismiss(
                                state,
                                http_client,
                                audio_tx,
                                ui,
                                ctx,
                                video,
                                &mut card_action,
                                true,
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
