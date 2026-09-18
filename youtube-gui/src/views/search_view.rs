use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::Video;

use crate::types::{AppState, PendingAction, PlayerCommand};
use crate::views::draw_video_card;

pub fn render_search_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    search_query: &mut String,
    results: &Option<Result<Vec<Video>, String>>,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("🔍 Search YouTube")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(20.0);
        let response = ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Search videos, channels...")
                .desired_width(300.0),
        );
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let q = search_query.trim().to_string();
            if !q.is_empty() {
                action = Some(PendingAction::Search { query: q });
            }
        }
        if ui.button("Search").clicked() {
            let q = search_query.trim().to_string();
            if !q.is_empty() {
                action = Some(PendingAction::Search { query: q });
            }
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    match results {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Enter a search term above and press Search or Enter.");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Search failed");
                ui.add_space(10.0);
                ui.label(err_msg);
            });
        }
        Some(Ok(vids)) => {
            if vids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No search results found.");
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
