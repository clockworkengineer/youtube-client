use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::Playlist;

use crate::types::{AppState, PendingAction};
use crate::views::{get_or_fetch_thumbnail, render_error_banner};

pub fn render_playlists_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    playlists: &Option<Result<Arc<Vec<Playlist>>, String>>,
    title_input: &mut String,
    desc_input: &mut String,
) -> Option<PendingAction> {
    let mut action = None;

    let confirm_del_id = ui.make_persistent_id("confirm_delete_playlist_id");
    let mut pending_delete_id: Option<String> = ui
        .data_mut(|d| d.get_temp::<Option<String>>(confirm_del_id))
        .flatten();

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("📂 Your Playlists")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        if let Some(Ok(lists)) = playlists {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("({} playlists)", lists.len()))
                    .size(13.0)
                    .color(egui::Color32::GRAY),
            );
        }
        ui.add_space(16.0);
        if ui
            .button("🔄 Refresh")
            .on_hover_text("Reload your YouTube playlists (F5)")
            .clicked()
        {
            action = Some(PendingAction::RetryPlaylists);
        }
    });
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
                    .hint_text("Required playlist title...")
                    .desired_width(180.0),
            );
            ui.label("Description:");
            ui.add(
                egui::TextEdit::singleline(desc_input)
                    .hint_text("Optional description...")
                    .desired_width(200.0),
            );
            let can_create = !title_input.trim().is_empty();
            if ui
                .add_enabled(
                    can_create,
                    egui::Button::new(
                        egui::RichText::new("➕ Create")
                            .strong()
                            .color(egui::Color32::from_rgb(100, 220, 100)),
                    ),
                )
                .on_hover_text("Create a new private playlist on your YouTube account")
                .on_disabled_hover_text("Enter a playlist title first")
                .clicked()
            {
                let t = title_input.trim().to_string();
                let d = desc_input.trim().to_string();
                let desc = if d.is_empty() { None } else { Some(d) };
                action = Some(PendingAction::CreatePlaylist {
                    title: t,
                    description: desc,
                });
                title_input.clear();
                desc_input.clear();
            }
        });
    });
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(10.0);

    match playlists {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.spinner();
                ui.label("Loading your playlists...");
            });
        }
        Some(Err(err_msg)) => {
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Failed to load playlists",
                err_msg,
                PendingAction::RetryPlaylists,
                &mut banner_action,
            );
            if !matches!(banner_action, PendingAction::None) {
                action = Some(banner_action);
            }
        }
        Some(Ok(lists)) => {
            if lists.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No playlists found on your account. Create one above!");
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for list in lists.iter() {
                            ui.push_id(&list.id, |ui| {
                                let texture = get_or_fetch_thumbnail(
                                    state,
                                    http_client,
                                    ctx,
                                    &list.id,
                                    &list.thumbnail_url,
                                );
                                let mut interact_with_card = true;

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
                                                egui::vec2(80.0, 60.0),
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
                                                egui::FontId::proportional(28.0),
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
                                                    "🎬 {} videos",
                                                    list.video_count
                                                ))
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(160, 160, 170)),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let is_confirming = pending_delete_id.as_deref()
                                                    == Some(list.id.as_str());
                                                if is_confirming {
                                                    interact_with_card = false;
                                                    if ui.button("Cancel").clicked() {
                                                        pending_delete_id = None;
                                                    }
                                                    if ui
                                                        .button(
                                                            egui::RichText::new("✔ Confirm Delete")
                                                                .color(egui::Color32::from_rgb(
                                                                    255, 90, 90,
                                                                ))
                                                                .strong(),
                                                        )
                                                        .clicked()
                                                    {
                                                        pending_delete_id = None;
                                                        action =
                                                            Some(PendingAction::DeletePlaylist {
                                                                playlist_id: list.id.clone(),
                                                            });
                                                    }
                                                    ui.label(
                                                        egui::RichText::new("Delete playlist?")
                                                            .size(12.0)
                                                            .color(egui::Color32::from_rgb(
                                                                255, 180, 100,
                                                            )),
                                                    );
                                                } else if ui
                                                    .button(egui::RichText::new("🗑 Delete").color(
                                                        egui::Color32::from_rgb(255, 110, 110),
                                                    ))
                                                    .on_hover_text(
                                                        "Delete this playlist from YouTube",
                                                    )
                                                    .clicked()
                                                {
                                                    interact_with_card = false;
                                                    pending_delete_id = Some(list.id.clone());
                                                }
                                            },
                                        );
                                    });
                                });

                                if interact_with_card {
                                    let click_resp = ui
                                        .interact(
                                            response.response.rect,
                                            response.response.id,
                                            egui::Sense::click(),
                                        )
                                        .on_hover_text(format!(
                                            "Open playlist '{}' (ID: {})",
                                            list.title, list.id
                                        ));
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

    ui.data_mut(|d| d.insert_temp(confirm_del_id, pending_delete_id));

    action
}
