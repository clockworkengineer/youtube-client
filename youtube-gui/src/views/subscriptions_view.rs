use std::sync::{Arc, Mutex};
use eframe::egui;
use youtube_client_lib::Subscription;

use crate::types::{AppState, PendingAction};
use crate::views::get_or_fetch_thumbnail;

pub fn render_subscriptions_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    subscriptions: &Option<Result<Vec<Subscription>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.heading(
            egui::RichText::new("📺 YouTube Premium Subscriptions")
                .size(24.0)
                .strong()
                .color(egui::Color32::from_rgb(255, 60, 60)),
        );
        ui.label(
            egui::RichText::new("A portable native client showing your YouTube subscriptions")
                .size(13.0)
                .color(egui::Color32::from_rgb(150, 150, 160)),
        );
        ui.add_space(15.0);
        ui.separator();
    });

    ui.add_space(10.0);

    match subscriptions {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Connecting to YouTube and loading your subscriptions...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Error Encountered");
                ui.add_space(10.0);
                ui.label(err_msg);
                ui.add_space(20.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetrySubscriptions);
                }
            });
        }
        Some(Ok(subs)) => {
            if subs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No subscriptions found on your YouTube account.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for sub in subs {
                            ui.push_id(&sub.channel_id, |ui| {
                                let texture = get_or_fetch_thumbnail(state, http_client, ctx, &sub.channel_id, &sub.thumbnail_url);

                                let response = ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = &texture {
                                            ui.add(egui::Image::from_texture(tex).max_width(50.0).max_height(50.0));
                                        } else {
                                            let (rect, _response) = ui.allocate_exact_size(
                                                egui::vec2(50.0, 50.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                4.0,
                                                egui::Color32::from_rgb(50, 53, 60),
                                            );
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "📷",
                                                egui::FontId::proportional(20.0),
                                                egui::Color32::LIGHT_GRAY,
                                            );
                                        }

                                        ui.add_space(10.0);

                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&sub.title)
                                                    .size(16.0)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.add_space(2.0);
                                            ui.label(
                                                egui::RichText::new(format!("Channel ID: {}", sub.channel_id))
                                                    .size(12.0)
                                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                                            );
                                        });
                                    });
                                });

                                let response = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                                if response.clicked() {
                                    action = Some(PendingAction::LoadChannel {
                                        id: sub.channel_id.clone(),
                                        title: sub.title.clone(),
                                        description: sub.description.clone(),
                                    });
                                }

                                if response.hovered() {
                                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                }

                                ui.add_space(8.0);
                            });
                        }
                    });
            }
        }
    }

    action
}
