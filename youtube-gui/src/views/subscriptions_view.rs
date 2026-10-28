use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::Subscription;

use crate::types::{AppState, PendingAction};
use crate::views::{get_or_fetch_thumbnail, render_error_banner};

pub fn render_subscriptions_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    subscriptions: &Option<Result<Arc<Vec<Subscription>>, String>>,
    filter_input: &mut String,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("📺 Subscriptions")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        if let Some(Ok(subs)) = subscriptions {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("({} channels)", subs.len()))
                    .size(13.0)
                    .color(egui::Color32::GRAY),
            );
        }
        ui.add_space(15.0);
        if ui
            .button("🔄 Refresh")
            .on_hover_text("Reload your YouTube subscriptions (F5)")
            .clicked()
        {
            action = Some(PendingAction::RetrySubscriptions);
        }
    });
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("🔎 Filter:");
        ui.add(
            egui::TextEdit::singleline(filter_input)
                .hint_text("Filter subscribed channels by name or description...")
                .desired_width(280.0),
        );
        if !filter_input.is_empty()
            && ui
                .button("✖ Clear")
                .on_hover_text("Clear filter text")
                .clicked()
        {
            filter_input.clear();
        }
    });
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    match subscriptions {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Connecting to YouTube and loading your subscriptions...");
            });
        }
        Some(Err(err_msg)) => {
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Could not load subscriptions",
                err_msg,
                PendingAction::RetrySubscriptions,
                &mut banner_action,
            );
            if !matches!(banner_action, PendingAction::None) {
                action = Some(banner_action);
            }
        }
        Some(Ok(subs)) => {
            let filter = filter_input.trim().to_lowercase();
            let filtered_subs: Vec<&Subscription> = if filter.is_empty() {
                subs.iter().collect()
            } else {
                subs.iter()
                    .filter(|s| {
                        s.title.to_lowercase().contains(&filter)
                            || s.description.to_lowercase().contains(&filter)
                            || s.channel_id.to_lowercase().contains(&filter)
                    })
                    .collect()
            };

            if subs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No subscriptions found on your YouTube account.");
                });
            } else if filtered_subs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(format!("No subscriptions matching \"{filter_input}\"."));
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for sub in filtered_subs {
                            ui.push_id(&sub.channel_id, |ui| {
                                let texture = get_or_fetch_thumbnail(
                                    state,
                                    http_client,
                                    ctx,
                                    &sub.channel_id,
                                    &sub.thumbnail_url,
                                );

                                let response = ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = &texture {
                                            ui.add(
                                                egui::Image::from_texture(tex)
                                                    .max_width(50.0)
                                                    .max_height(50.0)
                                                    .rounding(25.0),
                                            );
                                        } else {
                                            let (rect, _response) = ui.allocate_exact_size(
                                                egui::vec2(50.0, 50.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                25.0,
                                                egui::Color32::from_rgb(50, 53, 60),
                                            );
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "📺",
                                                egui::FontId::proportional(20.0),
                                                egui::Color32::LIGHT_GRAY,
                                            );
                                        }

                                        ui.add_space(12.0);

                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&sub.title)
                                                    .size(16.0)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.add_space(2.0);
                                            let snippet = truncate_one_line(&sub.description, 110);
                                            if !snippet.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(snippet)
                                                        .size(12.0)
                                                        .color(egui::Color32::from_rgb(
                                                            160, 165, 175,
                                                        )),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Click to view channel uploads & stats",
                                                    )
                                                    .size(11.5)
                                                    .color(egui::Color32::from_rgb(130, 135, 145)),
                                                );
                                            }
                                        });
                                    });
                                });

                                let response = ui
                                    .interact(
                                        response.response.rect,
                                        response.response.id,
                                        egui::Sense::click(),
                                    )
                                    .on_hover_text(format!(
                                        "Open {} (Channel ID: {})",
                                        sub.title, sub.channel_id
                                    ));
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

                                ui.add_space(6.0);
                            });
                        }
                    });
            }
        }
    }

    action
}

fn truncate_one_line(text: &str, max_chars: usize) -> String {
    let single_line = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if single_line.chars().count() <= max_chars {
        single_line
    } else {
        let truncated: String = single_line.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}
