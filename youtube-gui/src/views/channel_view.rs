use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::{Subscription, Video};

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::draw_video_card;

pub fn render_channel_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    channel_id: &str,
    channel_title: &str,
    channel_description: &str,
    videos: &Option<Result<Vec<Video>, String>>,
    subscriptions: &Option<Result<Vec<Subscription>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("⬅ Go Back").clicked() {
                action = Some(PendingAction::GoBack);
            }
            ui.add_space(15.0);
            ui.heading(
                egui::RichText::new(format!("Videos: {channel_title}"))
                    .size(20.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );

            ui.add_space(20.0);
            if let Some(sub_details) = if let Some(Ok(subs)) = subscriptions {
                subs.iter().find(|sub| sub.channel_id == channel_id)
            } else {
                None
            } {
                if ui
                    .button("✓ Subscribed")
                    .on_hover_text("Click to unsubscribe")
                    .clicked()
                {
                    action = Some(PendingAction::Unsubscribe {
                        subscription_id: sub_details.id.clone(),
                    });
                }
            } else if ui.button("➕ Subscribe").clicked() {
                action = Some(PendingAction::Subscribe {
                    channel_id: channel_id.to_string(),
                });
            }
        });
        if !channel_description.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(channel_description)
                    .size(13.0)
                    .color(egui::Color32::from_rgb(160, 160, 170)),
            );
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    match videos {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Loading recent uploads from channel...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Failed to load videos",
                );
                ui.add_space(10.0);
                ui.label(err_msg);
                ui.add_space(20.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetryVideos {
                        id: channel_id.to_string(),
                        title: channel_title.to_string(),
                        description: channel_description.to_string(),
                    });
                }
            });
        }
        Some(Ok(vids)) => {
            if vids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No uploads found on this channel.");
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
