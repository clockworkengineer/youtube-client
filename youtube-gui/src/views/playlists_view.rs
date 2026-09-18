use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::Playlist;

use crate::types::{AppState, PendingAction};
use crate::views::get_or_fetch_thumbnail;

pub fn render_playlists_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlists: &Option<Result<Vec<Playlist>, String>>,
    title_input: &mut String,
    desc_input: &mut String,
) -> Option<PendingAction> {
    let mut action = None;

    ui.heading("📂 Your Playlists");
    ui.add_space(10.0);

    // Create New Playlist Form
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("➕ Create New Playlist")
                .strong()
                .size(14.0),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Title:");
            ui.add(
                egui::TextEdit::singleline(title_input)
                    .hint_text("Playlist title...")
                    .desired_width(180.0),
            );
            ui.label("Description (opt):");
            ui.add(
                egui::TextEdit::singleline(desc_input)
                    .hint_text("Optional description...")
                    .desired_width(200.0),
            );
            if ui
                .button(
                    egui::RichText::new("Create")
                        .strong()
                        .color(egui::Color32::from_rgb(100, 220, 100)),
                )
                .clicked()
            {
                let t = title_input.trim().to_string();
                if !t.is_empty() {
                    let d = desc_input.trim().to_string();
                    let desc = if d.is_empty() { None } else { Some(d) };
                    action = Some(PendingAction::CreatePlaylist {
                        title: t,
                        description: desc,
                    });
                    title_input.clear();
                    desc_input.clear();
                }
            }
        });
    });
    ui.add_space(15.0);

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
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Failed to load playlists",
                );
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
                                let texture = get_or_fetch_thumbnail(
                                    state,
                                    http_client,
                                    ctx,
                                    &list.id,
                                    &list.thumbnail_url,
                                );
                                let mut delete_clicked = false;

                                let response = ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = &texture {
                                            ui.add(
                                                egui::Image::from_texture(tex)
                                                    .max_width(80.0)
                                                    .max_height(80.0),
                                            );
                                        } else {
                                            let (rect, _response) = ui.allocate_exact_size(
                                                egui::vec2(80.0, 80.0),
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
                                                egui::RichText::new(format!(
                                                    "Videos: {}",
                                                    list.video_count
                                                ))
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(160, 160, 170)),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(egui::RichText::new("🗑 Delete").color(
                                                        egui::Color32::from_rgb(255, 100, 100),
                                                    ))
                                                    .clicked()
                                                {
                                                    delete_clicked = true;
                                                }
                                            },
                                        );
                                    });
                                });

                                if delete_clicked {
                                    action = Some(PendingAction::DeletePlaylist {
                                        playlist_id: list.id.clone(),
                                    });
                                } else {
                                    let click_resp = ui.interact(
                                        response.response.rect,
                                        response.response.id,
                                        egui::Sense::click(),
                                    );
                                    if click_resp.clicked() {
                                        action = Some(PendingAction::LoadPlaylistVideos {
                                            id: list.id.clone(),
                                            title: list.title.clone(),
                                        });
                                    }
                                    if click_resp.hovered() {
                                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
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
