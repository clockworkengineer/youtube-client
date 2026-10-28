use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::{ChannelDetails, Subscription, Video};

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::{draw_video_card, get_or_fetch_thumbnail, render_error_banner};

#[allow(clippy::too_many_arguments)]
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

    let unsub_confirm_id = ui.make_persistent_id(format!("confirm_unsub_{channel_id}"));
    let mut confirm_unsub = ui
        .data_mut(|d| d.get_temp::<bool>(unsub_confirm_id))
        .unwrap_or(false);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui
                .button("⬅ Back")
                .on_hover_text("Return to previous screen (Esc / Alt+Left)")
                .clicked()
            {
                action = Some(PendingAction::GoBack);
            }
            ui.add_space(12.0);

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
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .on_hover_text(format!("Channel ID: {channel_id}"));
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

            ui.add_space(16.0);
            if let Some(sub_details) = if let Some(Ok(subs)) = subscriptions {
                subs.iter().find(|sub| sub.channel_id == channel_id)
            } else {
                None
            } {
                if confirm_unsub {
                    ui.label(
                        egui::RichText::new("Unsubscribe?")
                            .color(egui::Color32::from_rgb(255, 180, 100)),
                    );
                    if ui
                        .button(
                            egui::RichText::new("✔ Yes")
                                .color(egui::Color32::from_rgb(255, 100, 100))
                                .strong(),
                        )
                        .clicked()
                    {
                        confirm_unsub = false;
                        action = Some(PendingAction::Unsubscribe {
                            subscription_id: sub_details.id.clone(),
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        confirm_unsub = false;
                    }
                } else if ui
                    .button("✓ Subscribed")
                    .on_hover_text("Click to unsubscribe from this channel")
                    .clicked()
                {
                    confirm_unsub = true;
                }
            } else if ui
                .button(
                    egui::RichText::new("➕ Subscribe")
                        .strong()
                        .color(egui::Color32::from_rgb(255, 110, 110)),
                )
                .on_hover_text("Subscribe to this channel")
                .clicked()
            {
                action = Some(PendingAction::Subscribe {
                    channel_id: channel_id.to_string(),
                });
            }

            if ui
                .button("🔄 Refresh")
                .on_hover_text("Reload channel uploads (F5)")
                .clicked()
            {
                action = Some(PendingAction::RetryVideos {
                    id: channel_id.to_string(),
                    title: channel_title.to_string(),
                    description: channel_description.to_string(),
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

    ui.data_mut(|d| d.insert_temp(unsub_confirm_id, confirm_unsub));

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
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Failed to load channel videos",
                err_msg,
                PendingAction::RetryVideos {
                    id: channel_id.to_string(),
                    title: channel_title.to_string(),
                    description: channel_description.to_string(),
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
