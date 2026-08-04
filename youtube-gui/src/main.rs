mod actions;
mod player;
mod types;

use actions::*;
use types::*;

use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::{
    launch_external_player, load_string_set_from_file, save_string_set_to_file, scan_downloads_dir,
    DownloadStatus,
};

struct YoutubeGuiApp {
    state: Arc<Mutex<AppState>>,
    http_client: reqwest::Client,
    client_id_input: String,
    client_secret_input: String,
    search_input: String,
    audio_tx: std::sync::mpsc::Sender<PlayerCommand>,
}



impl YoutubeGuiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize the styling to make it look premium
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 27, 30);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(33, 37, 43);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(41, 46, 54);
        cc.egui_ctx.set_visuals(visuals);

        let config = youtube_client_lib::load_config();
        let client_id_input = config.client_id.unwrap_or_default();
        let client_secret_input = config.client_secret.unwrap_or_default();
        let downloads_dir = PathBuf::from(config.downloads_dir.unwrap_or_else(|| "downloads".to_string()));

        let mut downloads = HashMap::new();
        if downloads_dir.exists() {
            scan_downloads_dir(&downloads_dir, &mut downloads);
        }

        let cleared_video_ids = load_string_set_from_file(std::path::Path::new("cleared_videos.json"));
        let state = Arc::new(Mutex::new(AppState {
            subscriptions: None,
            new_videos: None,
            cleared_video_ids,
            thumbnails: HashMap::new(),
            current_view: View::Subscriptions,
            view_history: Vec::new(),
            logging_in: false,
            login_error: None,
            downloads,
            player_state: PlayerState {
                current_title: String::new(),
                playing: false,
            },
            playlists: None,
            playlist_action_status: None,
            downloads_dir,
        }));

        let http_client = reqwest::Client::new();

        let (audio_tx, audio_rx) = std::sync::mpsc::channel::<PlayerCommand>();
        player::spawn_audio_worker(state.clone(), audio_rx, cc.egui_ctx.clone());

        // Initialize YoutubeClient and fetch subscriptions asynchronously
        spawn_fetch_subscriptions(state.clone(), cc.egui_ctx.clone());

        Self {
            state,
            http_client,
            client_id_input,
            client_secret_input,
            search_input: String::new(),
            audio_tx,
        }
    }

    fn get_or_fetch_thumbnail(&self, ctx: &egui::Context, id: &str, url: &str) -> Option<egui::TextureHandle> {
        let mut start_fetch = false;
        let texture = {
            let mut s = self.state.lock().unwrap();
            let thumbnail_entry = s.thumbnails.entry(id.to_string()).or_insert_with(|| Thumbnail {
                texture: None,
                loading: false,
            });

            if thumbnail_entry.texture.is_none() && !thumbnail_entry.loading && !url.is_empty() {
                thumbnail_entry.loading = true;
                start_fetch = true;
            }
            thumbnail_entry.texture.clone()
        };

        if start_fetch {
            fetch_thumbnail(
                ctx.clone(),
                self.state.clone(),
                self.http_client.clone(),
                id.to_string(),
                url.to_string(),
            );
        }
        texture
    }

    fn draw_video_card_with_dismiss(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        video: &youtube_client_lib::Video,
        action: &mut PendingAction,
        can_dismiss: bool,
    ) {
        ui.push_id(&video.id, |ui| {
            let texture = self.get_or_fetch_thumbnail(ctx, &video.id, &video.thumbnail_url);
            let (download_status, player_state) = {
                let s_lock = self.state.lock().unwrap();
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
                                            let _ = self.audio_tx.send(PlayerCommand::Stop);
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

    fn draw_video_card(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        video: &youtube_client_lib::Video,
        action: &mut PendingAction,
    ) {
        self.draw_video_card_with_dismiss(ui, ctx, video, action, false);
    }
}

enum PendingAction {
    None,
    SpawnLogin { id: String, secret: String },
    RetrySubscriptions,
    GoToSubscriptions,
    GoToNewVideos,
    LoadNewVideos,
    ClearAllNewVideos,
    DismissNewVideo { video_id: String },
    ResetClearedVideos,
    LoadChannel { id: String, title: String, description: String },
    GoBack,
    RetryVideos { id: String, title: String, description: String },
    SpawnDownload { video: youtube_client_lib::Video, is_audio: bool },
    PlayLocal { path: PathBuf, title: String },
    StreamVideo { video_id: String },
    Search { query: String },
    RetrySearch { query: String },
    LoadPlaylists,
    LoadPlaylistVideos { id: String, title: String },
    RetryPlaylists,
    RetryPlaylistVideos { id: String, title: String },
    LoadVideoDetails { video: youtube_client_lib::Video },
    RetryVideoDetails { video: youtube_client_lib::Video },
    Subscribe { channel_id: String },
    Unsubscribe { subscription_id: String },
    RateVideo { video_id: String, rating: String },
    AddToPlaylist { playlist_id: String, playlist_title: String, video_id: String },
}

impl eframe::App for YoutubeGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (current_view, subscriptions, logging_in, login_error, player_state, playlists, playlist_action_status, new_videos, cleared_video_count) = {
            let s = self.state.lock().unwrap();
            (
                s.current_view.clone(),
                s.subscriptions.clone(),
                s.logging_in,
                s.login_error.clone(),
                s.player_state.clone(),
                s.playlists.clone(),
                s.playlist_action_status.clone(),
                s.new_videos.clone(),
                s.cleared_video_ids.len(),
            )
        };
        let mut action = PendingAction::None;

        if !matches!(current_view, View::Login) && !player_state.current_title.is_empty() {
            egui::TopBottomPanel::bottom("audio_player").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎵 Playing:").strong());
                    ui.label(&player_state.current_title);
                    
                    ui.add_space(20.0);

                    if player_state.playing {
                        if ui.button("⏸ Pause").clicked() {
                            let _ = self.audio_tx.send(PlayerCommand::Pause);
                        }
                    } else {
                        if ui.button("▶ Resume").clicked() {
                            let _ = self.audio_tx.send(PlayerCommand::Resume);
                        }
                    }

                    if ui.button("⏹ Stop").clicked() {
                        let _ = self.audio_tx.send(PlayerCommand::Stop);
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if !matches!(current_view, View::Login) {
                ui.horizontal(|ui| {
                    ui.label("🔍 Search YouTube:");
                    let response = ui.text_edit_singleline(&mut self.search_input);
                    if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Search").clicked() {
                        let q = self.search_input.trim().to_string();
                        if !q.is_empty() {
                            action = PendingAction::Search { query: q };
                        }
                    }
                });
                ui.add_space(5.0);

                // Navigation Tabs
                ui.horizontal(|ui| {
                    let on_subs = matches!(current_view, View::Subscriptions | View::ChannelVideos { .. });
                    if ui.selectable_label(on_subs, "📺 Subscriptions").clicked() {
                        action = PendingAction::GoToSubscriptions;
                    }
                    ui.add_space(10.0);
                    let on_new = matches!(current_view, View::NewVideos);
                    if ui.selectable_label(on_new, "🆕 New Videos").clicked() {
                        action = PendingAction::GoToNewVideos;
                    }
                    ui.add_space(10.0);
                    let on_playlists = matches!(current_view, View::Playlists { .. } | View::PlaylistVideos { .. });
                    if ui.selectable_label(on_playlists, "📂 Playlists").clicked() {
                        action = PendingAction::LoadPlaylists;
                    }
                });
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);
            }


            match current_view {
                View::Login => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.heading(
                            egui::RichText::new("🔐 YouTube OAuth Authentication")
                                .size(24.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 60, 60)),
                        );
                        ui.add_space(10.0);
                        ui.label("Please configure your Google OAuth2 credentials to authenticate the client.");
                        ui.add_space(20.0);
                    });

                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Google Client ID:");
                            ui.text_edit_singleline(&mut self.client_id_input);
                            ui.add_space(10.0);

                            ui.label("Google Client Secret:");
                            ui.text_edit_singleline(&mut self.client_secret_input);
                            ui.add_space(15.0);

                            if logging_in {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Attempting authentication & launching browser flow...");
                                });
                            } else {
                                if ui.button("Save & Login").clicked() {
                                    let id = self.client_id_input.trim().to_string();
                                    let secret = self.client_secret_input.trim().to_string();
                                    if !id.is_empty() && !secret.is_empty() {
                                        action = PendingAction::SpawnLogin { id, secret };
                                    }
                                }
                            }

                            let display_error = login_error.as_ref().or_else(|| {
                                if let Some(Err(err)) = &subscriptions {
                                    Some(err)
                                } else {
                                    None
                                }
                            });

                            if let Some(err) = display_error {
                                ui.add_space(10.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), format!("⚠️ Error: {}", err));
                            }
                        });
                    });
                }
                View::Subscriptions => {
                    // Title Header
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

                    match &subscriptions {
                        None => {
                            // Loading Indicator
                            ui.vertical_centered(|ui| {
                                ui.add_space(100.0);
                                ui.spinner();
                                ui.add_space(10.0);
                                ui.label("Connecting to YouTube and loading your subscriptions...");
                            });
                        }
                        Some(Err(err_msg)) => {
                            // Error Message Panel
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Error Encountered");
                                ui.add_space(10.0);
                                ui.label(err_msg);
                                ui.add_space(20.0);
                                if ui.button("Retry").clicked() {
                                    action = PendingAction::RetrySubscriptions;
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
                                // Scrollable List
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for sub in subs {
                                            ui.push_id(&sub.channel_id, |ui| {
                                                let texture = self.get_or_fetch_thumbnail(ctx, &sub.channel_id, &sub.thumbnail_url);

                                                // Display Card with nice layout
                                                let response = ui.group(|ui| {
                                                    ui.horizontal(|ui| {
                                                        // Thumbnail Render
                                                        if let Some(tex) = &texture {
                                                            ui.add(egui::Image::from_texture(tex).max_width(50.0).max_height(50.0));
                                                        } else {
                                                            // Placeholder thumbnail
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

                                                        // Text Info
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

                                                // Clicking logic to load channel videos
                                                let response = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                                                if response.clicked() {
                                                    action = PendingAction::LoadChannel {
                                                        id: sub.channel_id.clone(),
                                                        title: sub.title.clone(),
                                                        description: sub.description.clone(),
                                                    };
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
                }
                View::NewVideos => {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("🆕 New Videos Feed")
                                .size(22.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(20.0);
                        if ui.button("🔄 Refresh").clicked() {
                            action = PendingAction::LoadNewVideos;
                        }
                        ui.add_space(10.0);
                        if ui.button("🗑️ Clear All").on_hover_text("Clear all videos from New Videos feed").clicked() {
                            action = PendingAction::ClearAllNewVideos;
                        }
                        if cleared_video_count > 0 {
                            ui.add_space(10.0);
                            if ui.button(format!("↺ Reset Cleared ({})", cleared_video_count))
                                .on_hover_text("Restore all cleared/dismissed videos")
                                .clicked()
                            {
                                action = PendingAction::ResetClearedVideos;
                            }
                        }
                    });
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    match &new_videos {
                        None => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(100.0);
                                ui.spinner();
                                ui.add_space(10.0);
                                ui.label("Loading latest videos from your subscriptions...");
                            });
                        }
                        Some(Err(err_msg)) => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Failed to load new videos");
                                ui.add_space(10.0);
                                ui.label(err_msg);
                                ui.add_space(20.0);
                                if ui.button("Retry").clicked() {
                                    action = PendingAction::LoadNewVideos;
                                }
                            });
                        }
                        Some(Ok(vids)) => {
                            if vids.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label("No new videos available.");
                                });
                            } else {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for video in vids {
                                            self.draw_video_card_with_dismiss(ui, ctx, video, &mut action, true);
                                        }
                                    });
                            }
                        }
                    }
                }
                View::ChannelVideos { channel_id, channel_title, channel_description, videos } => {
                    // Header Area
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("⬅ Go Back").clicked() {
                                action = PendingAction::GoBack;
                            }
                            ui.add_space(15.0);
                            ui.heading(
                                egui::RichText::new(format!("Videos: {}", channel_title))
                                    .size(20.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );

                            ui.add_space(20.0);
                            if let Some(sub_details) = if let Some(Ok(subs)) = &subscriptions {
                                subs.iter().find(|sub| sub.channel_id == *channel_id)
                            } else {
                                None
                            } {
                                if ui.button("✓ Subscribed").on_hover_text("Click to unsubscribe").clicked() {
                                    action = PendingAction::Unsubscribe { subscription_id: sub_details.id.clone() };
                                }
                            } else {
                                if ui.button("➕ Subscribe").clicked() {
                                    action = PendingAction::Subscribe { channel_id: channel_id.clone() };
                                }
                            }
                        });
                        if !channel_description.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(&channel_description)
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
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Failed to load videos");
                                ui.add_space(10.0);
                                ui.label(err_msg);
                                ui.add_space(20.0);
                                if ui.button("Retry").clicked() {
                                    action = PendingAction::RetryVideos {
                                        id: channel_id.clone(),
                                        title: channel_title.clone(),
                                        description: channel_description.clone(),
                                    };
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
                                        for video in vids {
                                            self.draw_video_card(ui, ctx, &video, &mut action);
                                        }
                                    });
                            }
                        }
                    }
                }
                View::SearchResults { query, videos } => {
                    ui.horizontal(|ui| {
                        if ui.button("⬅ Go Back").clicked() {
                            action = PendingAction::GoBack;
                        }
                        ui.heading(format!("🔍 Search Results for: \"{}\"", query));
                    });
                    ui.add_space(10.0);

                    match videos {
                        None => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.spinner();
                                ui.label("Searching YouTube...");
                            });
                        }
                        Some(Err(err_msg)) => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.label(err_msg);
                                ui.add_space(20.0);
                                if ui.button("Retry").clicked() {
                                    action = PendingAction::RetrySearch { query: query.clone() };
                                }
                            });
                        }
                        Some(Ok(vids)) => {
                            if vids.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label("No videos found matching your query.");
                                });
                            } else {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for video in vids {
                                            self.draw_video_card(ui, ctx, &video, &mut action);
                                        }
                                    });
                            }
                        }
                    }
                }
                View::Playlists { playlists } => {
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
                                    action = PendingAction::RetryPlaylists;
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
                                                let texture = self.get_or_fetch_thumbnail(ctx, &list.id, &list.thumbnail_url);

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
                                                    action = PendingAction::LoadPlaylistVideos {
                                                        id: list.id.clone(),
                                                        title: list.title.clone(),
                                                    };
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
                }
                View::PlaylistVideos { playlist_id, playlist_title, videos } => {
                    ui.horizontal(|ui| {
                        if ui.button("⬅ Back to Playlists").clicked() {
                            action = PendingAction::LoadPlaylists;
                        }
                        ui.heading(format!("📂 Playlist: {}", playlist_title));
                    });
                    ui.add_space(10.0);

                    match videos {
                        None => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.spinner();
                                ui.label("Loading playlist videos...");
                            });
                        }
                        Some(Err(err_msg)) => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "⚠️ Failed to load playlist videos");
                                ui.add_space(10.0);
                                ui.label(err_msg);
                                ui.add_space(20.0);
                                if ui.button("Retry").clicked() {
                                    action = PendingAction::RetryPlaylistVideos {
                                        id: playlist_id.clone(),
                                        title: playlist_title.clone(),
                                    };
                                }
                            });
                        }
                        Some(Ok(vids)) => {
                            if vids.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label("No videos found in this playlist.");
                                });
                            } else {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for video in vids {
                                            self.draw_video_card(ui, ctx, &video, &mut action);
                                        }
                                    });
                            }
                        }
                    }
                }
                View::VideoDetails { video, comments } => {
                    ui.horizontal(|ui| {
                        if ui.button("⬅ Back").clicked() {
                            action = PendingAction::GoBack;
                        }
                        ui.heading("🎬 Video Details");
                    });
                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        let texture = self.get_or_fetch_thumbnail(ctx, &video.id, &video.thumbnail_url);
                        if let Some(tex) = &texture {
                            let img = egui::Image::from_texture(tex).max_width(200.0).max_height(150.0).sense(egui::Sense::click());
                            let img_response = ui.add(img);
                            if img_response.clicked() {
                                action = PendingAction::StreamVideo { video_id: video.id.clone() };
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
                            ui.label(
                                egui::RichText::new(format!("Published: {}", video.published_at))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("Video ID: {}", video.id))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                            );

                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                let download_status = {
                                    let s_lock = self.state.lock().unwrap();
                                    s_lock.downloads.get(&video.id).cloned().unwrap_or(DownloadStatus::NotStarted)
                                };

                                match &download_status {
                                    DownloadStatus::NotStarted => {
                                        if ui.button("📥 Download Video").clicked() {
                                            action = PendingAction::SpawnDownload { video: video.clone(), is_audio: false };
                                        }
                                        if ui.button("📥 Download Audio").clicked() {
                                            action = PendingAction::SpawnDownload { video: video.clone(), is_audio: true };
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
                                                    let _ = self.audio_tx.send(PlayerCommand::Stop);
                                                }
                                            } else {
                                                if ui.button(egui::RichText::new("▶ Play Local Audio").color(egui::Color32::from_rgb(100, 255, 100)).strong()).clicked() {
                                                    if let Some(path) = path_opt {
                                                        action = PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() };
                                                    }
                                                }
                                            }
                                        } else {
                                            if ui.button(egui::RichText::new("▶ Play Local Video").color(egui::Color32::from_rgb(100, 255, 100)).strong()).clicked() {
                                                if let Some(path) = path_opt {
                                                    action = PendingAction::PlayLocal { path: path.clone(), title: video.title.clone() };
                                                }
                                            }
                                        }
                                    }
                                    DownloadStatus::Failed(err) => {
                                        if ui.button("❌ Retry Video").clicked() {
                                            action = PendingAction::SpawnDownload { video: video.clone(), is_audio: false };
                                        }
                                        if ui.button("❌ Retry Audio").clicked() {
                                            action = PendingAction::SpawnDownload { video: video.clone(), is_audio: true };
                                        }
                                        ui.label(egui::RichText::new("Failed").color(egui::Color32::LIGHT_RED)).on_hover_text(err);
                                    }
                                }

                                ui.add_space(10.0);

                                if ui.button(egui::RichText::new("📺 Stream Video").color(egui::Color32::from_rgb(255, 100, 100)).strong()).clicked() {
                                    action = PendingAction::StreamVideo { video_id: video.id.clone() };
                                }

                                ui.add_space(15.0);
                                ui.separator();
                                ui.add_space(15.0);

                                ui.label("Rate video:");
                                if ui.button("👍 Like").clicked() {
                                    action = PendingAction::RateVideo { video_id: video.id.clone(), rating: "like".to_string() };
                                }
                                if ui.button("👎 Dislike").clicked() {
                                    action = PendingAction::RateVideo { video_id: video.id.clone(), rating: "dislike".to_string() };
                                }
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("📂 Add to Playlist:");
                                match &playlists {
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
                                                        action = PendingAction::AddToPlaylist {
                                                            playlist_id: playlist.id.clone(),
                                                            playlist_title: playlist.title.clone(),
                                                            video_id: video.id.clone(),
                                                        };
                                                    }
                                                }
                                            });
                                    }
                                }

                                if let Some(status) = &playlist_action_status {
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
                                    action = PendingAction::RetryVideoDetails { video: video.clone() };
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
                                                });
                                            });
                                            ui.add_space(5.0);
                                        }
                                    });
                            }
                        }
                    }
                }
            }
        });


        match action {
            PendingAction::None => {}
            PendingAction::SpawnLogin { id, secret } => {
                spawn_login_and_auth(self.state.clone(), ctx.clone(), id, secret);
            }
            PendingAction::RetrySubscriptions => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.subscriptions = None;
                }
                spawn_fetch_subscriptions(self.state.clone(), ctx.clone());
            }
            PendingAction::GoToSubscriptions => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.navigate_clear_history(View::Subscriptions);
            }
            PendingAction::GoToNewVideos => {
                let has_cache = {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.navigate_clear_history(View::NewVideos);
                    s_lock.new_videos.is_some()
                };
                if !has_cache {
                    spawn_fetch_new_videos(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::LoadNewVideos => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.new_videos = None;
                    s_lock.current_view = View::NewVideos;
                }
                spawn_fetch_new_videos(self.state.clone(), ctx.clone());
            }
            PendingAction::ClearAllNewVideos => {
                let mut s_lock = self.state.lock().unwrap();
                let ids_to_clear: Vec<String> = match &s_lock.new_videos {
                    Some(Ok(vids)) => vids.iter().map(|v| v.id.clone()).collect(),
                    _ => Vec::new(),
                };
                for id in ids_to_clear {
                    s_lock.cleared_video_ids.insert(id);
                }
                let _ = save_string_set_to_file(std::path::Path::new("cleared_videos.json"), &s_lock.cleared_video_ids);
                s_lock.new_videos = Some(Ok(Vec::new()));
            }
            PendingAction::DismissNewVideo { video_id } => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.cleared_video_ids.insert(video_id.clone());
                let _ = save_string_set_to_file(std::path::Path::new("cleared_videos.json"), &s_lock.cleared_video_ids);
                if let Some(Ok(ref mut vids)) = s_lock.new_videos {
                    vids.retain(|v| v.id != video_id);
                }
            }
            PendingAction::ResetClearedVideos => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.cleared_video_ids.clear();
                    let _ = save_string_set_to_file(std::path::Path::new("cleared_videos.json"), &s_lock.cleared_video_ids);
                    s_lock.new_videos = None;
                }
                spawn_fetch_new_videos(self.state.clone(), ctx.clone());
            }
            PendingAction::LoadChannel { id, title, description } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.navigate_to(View::ChannelVideos {
                        channel_id: id.clone(),
                        channel_title: title.clone(),
                        channel_description: description.clone(),
                        videos: None,
                    });
                }
                fetch_videos(ctx.clone(), self.state.clone(), id, title);
            }
            PendingAction::GoBack => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.go_back();
            }
            PendingAction::RetryVideos { id, title, description } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.current_view = View::ChannelVideos {
                        channel_id: id.clone(),
                        channel_title: title.clone(),
                        channel_description: description.clone(),
                        videos: None,
                    };
                }
                fetch_videos(ctx.clone(), self.state.clone(), id, title);
            }
            PendingAction::SpawnDownload { video, is_audio } => {
                spawn_download(self.state.clone(), ctx.clone(), video, is_audio);
            }
            PendingAction::PlayLocal { path, title } => {
                let is_mp3 = path.extension().map(|e| e == "mp3").unwrap_or(false);
                if is_mp3 {
                    println!("Playing local audio: {:?}", path);
                    let _ = self.audio_tx.send(PlayerCommand::Play(path, title));
                } else {
                    println!("Opening local video: {:?}", path);
                    if let Err(e) = launch_external_player(path.as_os_str()) {
                        println!("{} Falling back to default file opener.", e);
                        let _ = open::that(path);
                    }
                }
            }
            PendingAction::StreamVideo { video_id } => {
                let url = format!("https://www.youtube.com/watch?v={}", video_id);
                println!("Video clicked: {}", url);
                
                // Try to open the stream in MPV or VLC first
                if let Err(e) = launch_external_player(std::ffi::OsStr::new(&url)) {
                    println!("{} Falling back to default browser.", e);
                    let _ = open::that(url);
                }
            }
            PendingAction::Search { query } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.navigate_to(View::SearchResults {
                        query: query.clone(),
                        videos: None,
                    });
                }
                fetch_search_results(ctx.clone(), self.state.clone(), query);
            }
            PendingAction::RetrySearch { query } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.current_view = View::SearchResults {
                        query: query.clone(),
                        videos: None,
                    };
                }
                fetch_search_results(ctx.clone(), self.state.clone(), query);
            }
            PendingAction::LoadPlaylists => {
                let cache = {
                    let mut s_lock = self.state.lock().unwrap();
                    let cached = s_lock.playlists.clone();
                    s_lock.navigate_clear_history(View::Playlists { playlists: cached.clone() });
                    cached
                };
                if cache.is_none() {
                    spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::RetryPlaylists => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.playlists = None;
                    s_lock.current_view = View::Playlists { playlists: None };
                }
                spawn_fetch_playlists(self.state.clone(), ctx.clone());
            }
            PendingAction::LoadPlaylistVideos { id, title } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.navigate_to(View::PlaylistVideos {
                        playlist_id: id.clone(),
                        playlist_title: title.clone(),
                        videos: None,
                    });
                }
                fetch_playlist_videos(ctx.clone(), self.state.clone(), id, title);
            }
            PendingAction::RetryPlaylistVideos { id, title } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.current_view = View::PlaylistVideos {
                        playlist_id: id.clone(),
                        playlist_title: title.clone(),
                        videos: None,
                    };
                }
                fetch_playlist_videos(ctx.clone(), self.state.clone(), id, title);
            }
            PendingAction::LoadVideoDetails { video } => {
                let cache = {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.playlist_action_status = None;
                    s_lock.navigate_to(View::VideoDetails {
                        video: video.clone(),
                        comments: None,
                    });
                    s_lock.playlists.clone()
                };
                spawn_fetch_comments(self.state.clone(), ctx.clone(), video);
                if cache.is_none() {
                    spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::RetryVideoDetails { video } => {
                let cache = {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.playlist_action_status = None;
                    s_lock.current_view = View::VideoDetails {
                        video: video.clone(),
                        comments: None,
                    };
                    s_lock.playlists.clone()
                };
                spawn_fetch_comments(self.state.clone(), ctx.clone(), video);
                if cache.is_none() {
                    spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::Subscribe { channel_id } => {
                spawn_subscribe(self.state.clone(), ctx.clone(), channel_id);
            }
            PendingAction::Unsubscribe { subscription_id } => {
                spawn_unsubscribe(self.state.clone(), ctx.clone(), subscription_id);
            }
            PendingAction::RateVideo { video_id, rating } => {
                spawn_rate_video(self.state.clone(), ctx.clone(), video_id, rating);
            }
            PendingAction::AddToPlaylist { playlist_id, playlist_title, video_id } => {
                spawn_add_to_playlist(self.state.clone(), ctx.clone(), playlist_id, playlist_title, video_id);
            }
        }
    }
}


#[tokio::main]
async fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 700.0])
            .with_min_inner_size([400.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "YouTube Premium Subscriptions Client",
        native_options,
        Box::new(|cc| Box::new(YoutubeGuiApp::new(cc))),
    )
}

