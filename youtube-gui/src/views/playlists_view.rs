use std::sync::{Arc, Mutex};
use eframe::egui;
use youtube_client_lib::Playlist;

use crate::types::{AppState, PendingAction};
use crate::views::get_or_fetch_thumbnail;

pub fn render_playlists_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlists: &Option<Result<Vec<Playlist>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.heading("📂 Your Playlists");
    ui.add_space(10.0);

    match playlists {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.spinner();
                ui.label("Loading playlists...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Failed to load playlists");
                ui.add_space(10.0);
                ui.label(err_msg);
                ui.add_space(20.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetryPlaylists);
                }
            });
        }
        Some(Ok(lists)) => {
            if lists.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No playlists found on your account.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for list in lists {
                            ui.push_id(&list.id, |ui| {
                                let texture = get_or_fetch_thumbnail(state, http_client, ctx, &list.id, &list.thumbnail_url);

                                let response = ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = &texture {
                                            ui.add(egui::Image::from_texture(tex).max_width(80.0).max_height(80.0));
                                        } else {
                                            let (rect, _response) = ui.allocate_exact_size(
                                                egui::vec2(80.0, 80.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 53, 60));
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                "📂",
                                                egui::FontId::proportional(30.0),
                                                egui::Color32::LIGHT_GRAY,
                                            );
                                        }

                                        ui.add_space(15.0);

                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&list.title)
                                                    .size(16.0)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(format!("Videos: {}", list.video_count))
                                                    .size(12.0)
                                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                                            );
                                        });
                                    });
                                });

                                let click_resp = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                                if click_resp.clicked() {
                                    action = Some(PendingAction::LoadPlaylistVideos {
                                        id: list.id.clone(),
                                        title: list.title.clone(),
                                    });
                                }
                                if click_resp.hovered() {
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
