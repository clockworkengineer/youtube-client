//! # YouTube GUI Application Entry Point
//!
//! Initializes the eframe window, manages application lifecycle state, audio worker spawning,
//! and event dispatching.

mod actions;
mod player;
mod types;
mod views;

use actions::*;
use types::*;
use views::*;

use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::{
    launch_external_player, load_string_set_from_file, save_string_set_to_file, scan_downloads_dir,
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

        // Scan downloads directory for pre-existing media files
        let mut downloads = HashMap::new();
        if downloads_dir.exists() {
            scan_downloads_dir(&downloads_dir, &mut downloads);
        }

        // Restore dismissed video IDs from persistent JSON storage
        let cleared_video_ids = load_string_set_from_file(std::path::Path::new("cleared_videos.json"));
        let state = Arc::new(Mutex::new(AppState {
            subscriptions: None,
            new_videos: None,
            cleared_video_ids,
            thumbnails: HashMap::new(),
            thumbnail_lru: std::collections::VecDeque::new(),
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

        // Spawn Rodio background audio thread and channel listener
        let (audio_tx, audio_rx) = std::sync::mpsc::channel::<PlayerCommand>();
        player::spawn_audio_worker(state.clone(), audio_rx, cc.egui_ctx.clone());

        // Trigger background initial fetch of user's subscriptions
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
}

impl eframe::App for YoutubeGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (current_view, is_login, player_title, player_playing) = {
            let s = lock_state(&self.state);
            (
                s.current_view.clone(),
                matches!(s.current_view, View::Login),
                s.player_state.current_title.clone(),
                s.player_state.playing,
            )
        };
        let mut action = PendingAction::None;

        if !is_login && !player_title.is_empty() {
            egui::TopBottomPanel::bottom("audio_player").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎵 Playing:").strong());
                    ui.label(&player_title);
                    
                    ui.add_space(20.0);

                    if player_playing {
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
                    ui.add_space(10.0);
                    let on_about = matches!(current_view, View::About);
                    if ui.selectable_label(on_about, "ℹ️ About").clicked() {
                        action = PendingAction::GoToAbout;
                    }
                });
                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);
            }


            match current_view {
                View::Login => {
                    let s_lock = lock_state(&self.state);
                    if let Some(act) = render_login_view(ui, &mut self.client_id_input, &mut self.client_secret_input, &s_lock) {
                        action = act;
                    }
                }
                View::Subscriptions => {
                    let subs = {
                        let s = lock_state(&self.state);
                        s.subscriptions.clone()
                    };
                    if let Some(act) = render_subscriptions_view(&self.state, &self.http_client, ui, ctx, &subs) {
                        action = act;
                    }
                }
                View::NewVideos => {
                    let (vids, cleared_count) = {
                        let s = lock_state(&self.state);
                        (s.new_videos.clone(), s.cleared_video_ids.len())
                    };
                    if let Some(act) = render_new_videos_view(&self.state, &self.http_client, &self.audio_tx, ui, ctx, &vids, cleared_count) {
                        action = act;
                    }
                }
                View::ChannelVideos { channel_id, channel_title, channel_description, videos } => {
                    let subs = {
                        let s = lock_state(&self.state);
                        s.subscriptions.clone()
                    };
                    if let Some(act) = render_channel_view(&self.state, &self.http_client, &self.audio_tx, ui, ctx, &channel_id, &channel_title, &channel_description, &videos, &subs) {
                        action = act;
                    }
                }
                View::SearchResults { query: _, videos } => {
                    if let Some(act) = render_search_view(&self.state, &self.http_client, &self.audio_tx, ui, ctx, &mut self.search_input, &videos) {
                        action = act;
                    }
                }
                View::Playlists { playlists } => {
                    if let Some(act) = render_playlists_view(&self.state, &self.http_client, ui, ctx, &playlists) {
                        action = act;
                    }
                }
                View::PlaylistVideos { playlist_id, playlist_title, videos } => {
                    if let Some(act) = render_playlist_videos_view(&self.state, &self.http_client, &self.audio_tx, ui, ctx, &playlist_id, &playlist_title, &videos) {
                        action = act;
                    }
                }
                View::VideoDetails { video, comments } => {
                    let (p_state, pls, status) = {
                        let s = lock_state(&self.state);
                        (s.player_state.clone(), s.playlists.clone(), s.playlist_action_status.clone())
                    };
                    if let Some(act) = render_details_view(&self.state, &self.http_client, &self.audio_tx, ui, ctx, &video, &comments, &p_state, &pls, &status) {
                        action = act;
                    }
                }
                View::About => {
                    render_about_view(ui);
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
            PendingAction::GoToAbout => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.navigate_clear_history(View::About);
            }
        }
    }
}


fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

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

