//! # egui UI View & Video Card Renderers
//!
//! Provides egui view layout components, thumbnail texture caching, video cards,
//! and dismissible feed cards.

pub mod about_view;
pub mod channel_view;
pub mod details_view;
pub mod login_view;
pub mod new_videos_view;
pub mod playlist_videos_view;
pub mod playlists_view;
pub mod search_view;
pub mod subscriptions_view;
pub mod traits;

pub use about_view::*;
pub use channel_view::*;
pub use details_view::*;
pub use login_view::*;
pub use new_videos_view::*;
pub use playlist_videos_view::*;
pub use playlists_view::*;
pub use search_view::*;
pub use subscriptions_view::*;
pub use traits::*;

use eframe::egui;
use youtube_client_lib::utils::DownloadStatus;
use youtube_client_lib::Video;

use crate::actions::fetch_thumbnail;
use crate::types::{AppState, PendingAction, PlayerCommand, Thumbnail};

/// Render a single video card with optional dismiss button ("Clear").
pub fn draw_video_card_with_dismiss(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &std::sync::mpsc::Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    action: &mut PendingAction,
    can_dismiss: bool,
) {
    ui.push_id(&video.id, |ui| {
        let texture = get_or_fetch_thumbnail(state, http_client, ctx, &video.id, &video.thumbnail_url);
        let (download_status, player_state) = {
            let s_lock = state.lock().unwrap();
            (
                s_lock.downloads.get(&video.id).cloned().unwrap_or(DownloadStatus::NotStarted),
                s_lock.player_state.clone(),
            )
        };
        let mut card_clicked = false;

        let _response = ui.group(|ui| {
            ui.horizontal(|ui| {
                let left_response = ui.horizontal(|ui| {
                    if let Some(tex) = &texture {
                        ui.add(egui::Image::from_texture(tex).max_width(100.0).max_height(100.0));
                    } else {
                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(100.0, 100.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 53, 60));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "🎬",
                            egui::FontId::proportional(40.0),
                            egui::Color32::LIGHT_GRAY,
                        );
                    }

                    ui.add_space(15.0);

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&video.title)
                                .size(15.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let date = if video.published_at.len() >= 10 {
                                &video.published_at[..10]
                            } else {
                                &video.published_at
                            };
                            ui.label(
                                egui::RichText::new(format!("Published: {}", date))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(140, 140, 150)),
                            );
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new(format!("ID: {}", video.id))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(140, 140, 150)),
                            );
                        });
                    });
                });

                let left_interact = ui.interact(
                    left_response.response.rect,
                    left_response.response.id.with("click"),
                    egui::Sense::click(),
                );
                if left_interact.clicked() {
                    card_clicked = true;
                }
                if left_interact.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if can_dismiss {
                        if ui.button("❌ Clear").on_hover_text("Remove from New Videos feed").clicked() {
                            *action = PendingAction::DismissNewVideo { video_id: video.id.clone() };
                        }
                    }
                    match &download_status {
                        DownloadStatus::NotStarted => {
                            if ui.button("📥 Download Video").clicked() {
                                *action = PendingAction::SpawnDownload { video: video.clone(), is_audio: false };
                            }
                        }
                        DownloadStatus::Downloading { progress } => {
                            ui.spinner();
                            ui.label(progress);
                        }
                        DownloadStatus::Finished(_) => {
                            let path_opt = match &download_status {
                                DownloadStatus::Finished(path) => Some(path),
                                _ => None,
                            };
                            let is_mp3 = path_opt.map(|p| p.extension().map(|ext| ext == "mp3").unwrap_or(false)).unwrap_or(false);

                            if is_mp3 {
                                let is_playing = player_state.playing && player_state.current_title == video.title;
                                if is_playing {
                                    if ui.button(egui::RichText::new("⏹ Stop").color(egui::Color32::from_rgb(255, 100, 100)).strong()).clicked() {
                                        let _ = audio_tx.send(PlayerCommand::Stop);
                                    }
                                } else {
                                    if ui.button("▶ Play Local").clicked() {
                                        if let Some(path) = path_opt {
                                            *action = PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() };
                                        }
                                    }
                                }
                            } else {
                                if ui.button("▶ Play Local Video").clicked() {
                                    if let Some(path) = path_opt {
                                        *action = PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() };
                                    }
                                }
                            }
                        }
                        DownloadStatus::Failed(err) => {
                            if ui.button("❌ Retry").clicked() {
                                *action = PendingAction::SpawnDownload { video: video.clone(), is_audio: false };
                            }
                            ui.label(egui::RichText::new("Failed").color(egui::Color32::LIGHT_RED)).on_hover_text(err);
                        }
                    }
                });
            });
        });

        if card_clicked && matches!(action, PendingAction::None) {
            *action = PendingAction::LoadVideoDetails { video: video.clone() };
        }
    });
}

pub fn draw_video_card(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &std::sync::mpsc::Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    action: &mut PendingAction,
) {
    draw_video_card_with_dismiss(state, http_client, audio_tx, ui, ctx, video, action, false);
}

pub fn get_or_fetch_thumbnail(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    ctx: &egui::Context,
    id: &str,
    url: &str,
) -> Option<egui::TextureHandle> {
    let mut start_fetch = false;
    let texture = {
        let mut s = state.lock().unwrap();
        if !s.thumbnails.contains_key(id) {
            s.insert_thumbnail(
                id.to_string(),
                Thumbnail {
                    texture: None,
                    loading: false,
                },
            );
        }
        let thumbnail_entry = s.thumbnails.get_mut(id).unwrap();

        if thumbnail_entry.texture.is_none() && !thumbnail_entry.loading && !url.is_empty() {
            thumbnail_entry.loading = true;
            start_fetch = true;
        }
        thumbnail_entry.texture.clone()
    };

    if start_fetch {
        fetch_thumbnail(
            ctx.clone(),
            state.clone(),
            http_client.clone(),
            id.to_string(),
            url.to_string(),
        );
    }
    texture
}
