use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::{ChannelDetails, Subscription, Video};

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::{draw_video_card, get_or_fetch_thumbnail};

pub fn render_channel_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    channel_id: &str,
    channel_title: &str,
    channel_description: &str,
    channel_details: &Option<Result<Arc<ChannelDetails>, String>>,
    videos: &Option<Result<Arc<Vec<Video>>, String>>,
    subscriptions: &Option<Result<Arc<Vec<Subscription>>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("⬅ Go Back").clicked() {
                action = Some(PendingAction::GoBack);
            }
            ui.add_space(15.0);

            if let Some(Ok(details)) = channel_details {
                if !details.thumbnail_url.is_empty() {
                    if let Some(tex) = get_or_fetch_thumbnail(
                        state,
                        http_client,
                        ctx,
                        &format!("chan_{channel_id}"),
                        &details.thumbnail_url,
                    ) {
                        ui.add(
                            egui::Image::from_texture(&tex)
                                .max_width(48.0)
                                .max_height(48.0)
                                .rounding(24.0),
                        );
                        ui.add_space(8.0);
                    }
                }
            }

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new(format!("📺 {channel_title}"))
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    if let Some(Ok(details)) = channel_details {
                        if let Some(ref handle) = details.custom_url {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(handle)
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(150, 180, 220)),
                            );
                        }
                    }
                });

                if let Some(Ok(details)) = channel_details {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "👥 {} subscribers",
                                format_count(details.subscriber_count)
                            ))
                            .size(12.0)
                            .strong()
                            .color(egui::Color32::from_rgb(200, 210, 225)),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "🎬 {} videos",
                                format_count(details.video_count)
                            ))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(190, 195, 210)),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "👁 {} total views",
                                format_count(details.view_count)
                            ))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(190, 195, 210)),
                        );
                    });
                }
            });

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

        let desc = channel_details
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|d| d.description.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(channel_description);

        if !desc.is_empty() {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(desc)
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
                        for video in vids.iter() {
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

fn format_count(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
