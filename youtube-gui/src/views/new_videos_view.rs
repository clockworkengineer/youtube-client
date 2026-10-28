use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::Video;

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::{draw_video_card_with_dismiss, render_error_banner};

pub fn render_new_videos_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    new_videos: &Option<Result<Arc<Vec<Video>>, String>>,
    cleared_video_count: usize,
) -> Option<PendingAction> {
    let mut action = None;
    let has_undo = {
        let s = state.lock().unwrap();
        s.last_dismissed_video.is_some()
    };
    let has_videos = matches!(new_videos, Some(Ok(v)) if !v.is_empty());

    let confirm_id = ui.make_persistent_id("confirm_clear_all_new_videos");
    let mut confirm_clear_all = ui
        .data_mut(|d| d.get_temp::<bool>(confirm_id))
        .unwrap_or(false);

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("🆕 New Videos Feed")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        if let Some(Ok(vids)) = new_videos {
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
            .on_hover_text("Fetch latest videos from your subscriptions (F5)")
            .clicked()
        {
            action = Some(PendingAction::LoadNewVideos);
        }

        if has_undo {
            ui.add_space(6.0);
            if ui
                .button(
                    egui::RichText::new("↩ Undo Dismiss")
                        .color(egui::Color32::from_rgb(130, 210, 255)),
                )
                .on_hover_text("Restore the most recently dismissed video")
                .clicked()
            {
                action = Some(PendingAction::UndoDismissVideo);
            }
        }

        ui.add_space(6.0);
        if confirm_clear_all {
            ui.label(
                egui::RichText::new("Clear all videos?")
                    .color(egui::Color32::from_rgb(255, 180, 100)),
            );
            if ui
                .button(
                    egui::RichText::new("✔ Yes, Clear All")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .strong(),
                )
                .clicked()
            {
                confirm_clear_all = false;
                action = Some(PendingAction::ClearAllNewVideos);
            }
            if ui.button("Cancel").clicked() {
                confirm_clear_all = false;
            }
        } else if ui
            .add_enabled(has_videos, egui::Button::new("🗑 Clear All"))
            .on_hover_text("Dismiss all currently listed videos from the feed")
            .on_disabled_hover_text("Feed is already empty")
            .clicked()
        {
            confirm_clear_all = true;
        }

        if cleared_video_count > 0 {
            ui.add_space(6.0);
            if ui
                .button(format!("↺ Restore All ({cleared_video_count})"))
                .on_hover_text("Restore all previously dismissed videos back into the feed")
                .clicked()
            {
                action = Some(PendingAction::ResetClearedVideos);
            }
        }
    });

    ui.data_mut(|d| d.insert_temp(confirm_id, confirm_clear_all));

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
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Failed to load new videos feed",
                err_msg,
                PendingAction::LoadNewVideos,
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
                    ui.label("You're all caught up! No new videos in your feed.");
                    if cleared_video_count > 0 {
                        ui.add_space(8.0);
                        if ui
                            .button(format!(
                                "↺ Restore {cleared_video_count} Dismissed Videos"
                            ))
                            .clicked()
                        {
                            action = Some(PendingAction::ResetClearedVideos);
                        }
                    }
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for video in vids.iter() {
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
