use std::sync::{Arc, Mutex, mpsc::Sender};
use eframe::egui;
use youtube_client_lib::utils::DownloadStatus;
use youtube_client_lib::{Comment, Playlist, Video, VideoDetails};

use crate::types::{AppState, PendingAction, PlayerCommand, PlayerState};
use crate::views::get_or_fetch_thumbnail;

pub fn render_details_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    details: &Option<Result<VideoDetails, String>>,
    comments: &Option<Result<Vec<Comment>, String>>,
    player_state: &PlayerState,
    playlists: &Option<Result<Vec<Playlist>, String>>,
    playlist_action_status: &Option<Result<String, String>>,
    comment_input: &mut String,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        if ui.button("⬅ Back").clicked() {
            action = Some(PendingAction::GoBack);
        }
        ui.heading("🎬 Video Details");
    });
    ui.add_space(15.0);

    ui.horizontal(|ui| {
        let texture = get_or_fetch_thumbnail(state, http_client, ctx, &video.id, &video.thumbnail_url);
        if let Some(tex) = &texture {
            let img = egui::Image::from_texture(tex).max_width(200.0).max_height(150.0).sense(egui::Sense::click());
            let img_response = ui.add(img);
            if img_response.clicked() {
                action = Some(PendingAction::StreamVideo { video_id: video.id.clone() });
            }
            if img_response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                img_response.on_hover_text("Click to stream video");
            }
        } else {
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(200.0, 150.0),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 53, 60));
        }

        ui.add_space(20.0);

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(&video.title)
                    .size(18.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);

            // Channel link and published timestamp
            ui.horizontal(|ui| {
                if let Some(Ok(d)) = details {
                    if ui.link(egui::RichText::new(format!("👤 {}", d.channel_title)).strong().color(egui::Color32::from_rgb(120, 180, 255))).clicked() {
                        action = Some(PendingAction::LoadChannel {
                            id: d.channel_id.clone(),
                            title: d.channel_title.clone(),
                            description: String::new(),
                        });
                    }
                    ui.separator();
                } else if !video.channel_title.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("👤 {}", video.channel_title))
                            .color(egui::Color32::from_rgb(180, 180, 190)),
                    );
                    ui.separator();
                }

                ui.label(
                    egui::RichText::new(format!("Published: {}", video.published_at))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(160, 160, 170)),
                );
            });

            // Statistics badges if details are loaded
            if let Some(Ok(d)) = details {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("👁 {} views", d.view_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("👍 {} likes", d.like_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("⏱ {}", d.duration_formatted))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("💬 {} comments", d.comment_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                });
            }

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Video ID: {}", video.id))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(130, 130, 140)),
            );

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let download_status = {
                    let s_lock = state.lock().unwrap();
                    s_lock.downloads.get(&video.id).cloned().unwrap_or(DownloadStatus::NotStarted)
                };

                match &download_status {
                    DownloadStatus::NotStarted => {
                        if ui.button("📥 Download Video").clicked() {
                            action = Some(PendingAction::SpawnDownload { video: video.clone(), is_audio: false });
                        }
                        if ui.button("📥 Download Audio").clicked() {
                            action = Some(PendingAction::SpawnDownload { video: video.clone(), is_audio: true });
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
                                if ui.button(egui::RichText::new("⏹ Stop Audio").color(egui::Color32::from_rgb(255, 100, 100)).strong()).clicked() {
                                    let _ = audio_tx.send(PlayerCommand::Stop);
                                }
                            } else {
                                if ui.button(egui::RichText::new("▶ Play Local Audio").color(egui::Color32::from_rgb(100, 255, 100)).strong()).clicked() {
                                    if let Some(path) = path_opt {
                                        action = Some(PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() });
                                    }
                                }
                            }
                        } else {
                            if ui.button(egui::RichText::new("▶ Play Local Video").color(egui::Color32::from_rgb(100, 255, 100)).strong()).clicked() {
                                if let Some(path) = path_opt {
                                    action = Some(PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() });
                                }
                            }
                        }
                    }
                    DownloadStatus::Failed(err) => {
                        if ui.button("❌ Retry Video").clicked() {
                            action = Some(PendingAction::SpawnDownload { video: video.clone(), is_audio: false });
                        }
                        if ui.button("❌ Retry Audio").clicked() {
                            action = Some(PendingAction::SpawnDownload { video: video.clone(), is_audio: true });
                        }
                        ui.label(egui::RichText::new("Failed").color(egui::Color32::LIGHT_RED)).on_hover_text(err);
                    }
                }

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("📺 Stream Video").color(egui::Color32::from_rgb(255, 100, 100)).strong()).clicked() {
                        action = Some(PendingAction::StreamVideo { video_id: video.id.clone() });
                    }

                    if ui.button(egui::RichText::new("🌐 Watch in Browser").color(egui::Color32::from_rgb(100, 180, 255)).strong()).clicked() {
                        action = Some(PendingAction::OpenInBrowser {
                            url: format!("https://www.youtube.com/watch?v={}", video.id),
                        });
                    }
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(15.0);

                ui.label("Rate video:");
                if ui.button("👍 Like").clicked() {
                    action = Some(PendingAction::RateVideo { video_id: video.id.clone(), rating: "like".to_string() });
                }
                if ui.button("👎 Dislike").clicked() {
                    action = Some(PendingAction::RateVideo { video_id: video.id.clone(), rating: "dislike".to_string() });
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("📂 Add to Playlist:");
                match playlists {
                    None => {
                        ui.spinner();
                    }
                    Some(Err(_)) => {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "Failed to load playlists.");
                    }
                    Some(Ok(items)) => {
                        egui::ComboBox::from_id_source("add_to_playlist_cb")
                            .selected_text("Select Playlist...")
                            .show_ui(ui, |ui| {
                                for playlist in items {
                                    if ui.selectable_label(false, &playlist.title).clicked() {
                                        action = Some(PendingAction::AddToPlaylist {
                                            playlist_id: playlist.id.clone(),
                                            playlist_title: playlist.title.clone(),
                                            video_id: video.id.clone(),
                                        });
                                    }
                                }
                            });
                    }
                }

                if let Some(status) = playlist_action_status {
                    ui.add_space(10.0);
                    match status {
                        Ok(msg) => {
                            ui.colored_label(egui::Color32::from_rgb(100, 255, 100), msg);
                        }
                        Err(err) => {
                            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                        }
                    }
                }
            });
        });
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading("💬 Comments");
    ui.add_space(5.0);

    // Comment submission form
    ui.horizontal(|ui| {
        ui.label("Write a comment:");
        let response = ui.text_edit_singleline(comment_input);
        let submit = ui.button("💬 Post Comment").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if submit {
            let text = comment_input.trim().to_string();
            if !text.is_empty() {
                action = Some(PendingAction::PostComment {
                    video_id: video.id.clone(),
                    text,
                });
                comment_input.clear();
            }
        }
    });
    ui.add_space(10.0);

    match comments {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.spinner();
                ui.label("Loading comments...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Failed to load comments");
                ui.add_space(5.0);
                ui.label(err_msg);
                ui.add_space(10.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetryVideoDetails { video: video.clone() });
                }
            });
        }
        Some(Ok(items)) => {
            if items.is_empty() {
                ui.label("No comments found on this video.");
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for comment in items {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&comment.author_name)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(200, 200, 210)),
                                            );
                                            ui.add_space(10.0);
                                            ui.label(
                                                egui::RichText::new(format!("Likes: {}", comment.like_count))
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(140, 140, 150)),
                                            );
                                        });
                                        ui.add_space(4.0);
                                        ui.label(&comment.text_display);
                                    });
                                ui.add_space(5.0);
                            });
                        });
                    }
                });
            }
        }
    }

    action
}
