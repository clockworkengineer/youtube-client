#![windows_subsystem = "windows"]
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::large_enum_variant
)]
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
    append_to_log, launch_external_player_with_log, load_string_set_from_file,
    save_string_set_to_file, scan_downloads_dir,
};

struct YoutubeGuiApp {
    state: Arc<Mutex<AppState>>,
    http_client: reqwest::Client,
    client_id_input: String,
    client_secret_input: String,
    search_input: String,
    comment_input: String,
    playlist_title_input: String,
    playlist_desc_input: String,
    subscription_filter: String,
    settings_form: SettingsFormState,
    volume: f32,
    muted: bool,
    audio_tx: std::sync::mpsc::Sender<PlayerCommand>,
    window_pos: Option<[f32; 2]>,
    window_size: Option<[f32; 2]>,
    window_maximized: Option<bool>,
    window_dirty: bool,
    last_window_change: Option<std::time::Instant>,
}

impl YoutubeGuiApp {
    fn save_window_state(&mut self) {
        if !self.window_dirty {
            return;
        }
        let mut config = youtube_client_lib::load_config();
        config.window_pos = self.window_pos;
        config.window_size = self.window_size;
        config.window_maximized = self.window_maximized;
        config.volume = Some(self.volume);
        if youtube_client_lib::save_config(&config).is_ok() {
            self.window_dirty = false;
        }
    }

    fn new(cc: &eframe::CreationContext<'_>, log_file: PathBuf) -> Self {
        // Customize the styling to make it look premium
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 27, 30);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(33, 37, 43);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(41, 46, 54);
        cc.egui_ctx.set_visuals(visuals);

        let config = youtube_client_lib::load_config();
        let client_id_input = config.client_id.unwrap_or_default();
        let client_secret_input = config.client_secret.unwrap_or_default();
        let downloads_dir = PathBuf::from(
            config
                .downloads_dir
                .unwrap_or_else(|| "downloads".to_string()),
        );

        youtube_client_lib::utils::append_to_log(&log_file, "INFO", "youtube-gui started");

        // Scan downloads directory for pre-existing media files
        let downloads = HashMap::new();
        let cleared_videos_path = youtube_client_lib::resolve_app_data_path("cleared_videos.json");
        let cleared_video_ids = load_string_set_from_file(&cleared_videos_path);
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
                volume: 1.0,
                position_secs: 0.0,
                duration_secs: 0.0,
            },
            playlists: None,
            playlist_action_status: None,
            downloads_dir: downloads_dir.clone(),
            log_file: log_file.clone(),
            cleared_videos_path,
            toast: None,
        }));

        let http_client = reqwest::Client::new();

        // Spawn Rodio background audio thread and channel listener
        let (audio_tx, audio_rx) = std::sync::mpsc::channel::<PlayerCommand>();
        player::spawn_audio_worker(state.clone(), audio_rx, cc.egui_ctx.clone());

        // Trigger background initial fetch of user's subscriptions
        spawn_fetch_subscriptions(state.clone(), cc.egui_ctx.clone());

        // Background scan of downloads directory to keep startup latency sub-16ms
        let bg_state = state.clone();
        let bg_ctx = cc.egui_ctx.clone();
        let bg_downloads_dir = downloads_dir.clone();
        tokio::spawn(async move {
            if bg_downloads_dir.exists() {
                let mut scanned = HashMap::new();
                scan_downloads_dir(&bg_downloads_dir, &mut scanned);
                let mut s = lock_state(&bg_state);
                s.downloads.extend(scanned);
                bg_ctx.request_repaint();
            }
        });

        Self {
            state,
            http_client,
            client_id_input,
            client_secret_input,
            search_input: String::new(),
            comment_input: String::new(),
            playlist_title_input: String::new(),
            playlist_desc_input: String::new(),
            subscription_filter: String::new(),
            settings_form: SettingsFormState::default(),
            volume: config.volume.unwrap_or(1.0),
            muted: false,
            audio_tx,
            window_pos: config.window_pos,
            window_size: config.window_size,
            window_maximized: config.window_maximized,
            window_dirty: false,
            last_window_change: None,
        }
    }
}

impl Drop for YoutubeGuiApp {
    fn drop(&mut self) {
        self.save_window_state();
    }
}

impl eframe::App for YoutubeGuiApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_window_state();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Track window position, size, and maximized status for session restoration
        let (vp_pos, vp_size, vp_maximized, vp_minimized, close_requested) = ctx.input(|i| {
            let vp = i.viewport();
            (
                vp.outer_rect.map(|r| [r.min.x, r.min.y]),
                vp.inner_rect
                    .map(|r| [r.width(), r.height()])
                    .or_else(|| vp.outer_rect.map(|r| [r.width(), r.height()])),
                vp.maximized.unwrap_or(false),
                vp.minimized.unwrap_or(false),
                vp.close_requested(),
            )
        });

        if !vp_minimized {
            if vp_maximized {
                if self.window_maximized != Some(true) {
                    self.window_maximized = Some(true);
                    self.window_dirty = true;
                    self.last_window_change = Some(std::time::Instant::now());
                }
            } else {
                if self.window_maximized != Some(false) {
                    self.window_maximized = Some(false);
                    self.window_dirty = true;
                    self.last_window_change = Some(std::time::Instant::now());
                }
                if let Some(pos) = vp_pos {
                    if pos[0] > -10000.0
                        && pos[1] > -10000.0
                        && pos[0] < 50000.0
                        && pos[1] < 50000.0
                    {
                        if self.window_pos != Some(pos) {
                            self.window_pos = Some(pos);
                            self.window_dirty = true;
                            self.last_window_change = Some(std::time::Instant::now());
                        }
                    }
                }
                if let Some(size) = vp_size {
                    if size[0] >= 400.0 && size[1] >= 400.0 {
                        if self.window_size != Some(size) {
                            self.window_size = Some(size);
                            self.window_dirty = true;
                            self.last_window_change = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Periodically debounce save 2 seconds after window moves or resizes settle
        if self.window_dirty
            && self
                .last_window_change
                .is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(2))
        {
            self.save_window_state();
        }

        if close_requested && self.window_dirty {
            self.save_window_state();
        }
        let (current_view, is_login, player_title, player_playing, player_pos, player_dur) = {
            let s = lock_state(&self.state);
            (
                s.current_view.clone(),
                matches!(s.current_view, View::Login),
                s.player_state.current_title.clone(),
                s.player_state.playing,
                s.player_state.position_secs,
                s.player_state.duration_secs,
            )
        };
        let mut action = PendingAction::None;

        if !is_login && !player_title.is_empty() {
            if player_playing {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }

            egui::TopBottomPanel::bottom("audio_player").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎵 Playing:").strong());
                    ui.label(&player_title);

                    ui.add_space(15.0);

                    if ui.button("⏪ 10s").clicked() {
                        let _ = self.audio_tx.send(PlayerCommand::Skip(-10));
                    }

                    if player_playing {
                        if ui.button("⏸ Pause").clicked() {
                            let _ = self.audio_tx.send(PlayerCommand::Pause);
                        }
                    } else if ui.button("▶ Resume").clicked() {
                        let _ = self.audio_tx.send(PlayerCommand::Resume);
                    }

                    if ui.button("⏩ 10s").clicked() {
                        let _ = self.audio_tx.send(PlayerCommand::Skip(10));
                    }

                    if ui.button("⏹ Stop").clicked() {
                        let _ = self.audio_tx.send(PlayerCommand::Stop);
                    }

                    ui.add_space(15.0);

                    // Scrubber and time
                    let format_time = |secs: f32| -> String {
                        let total = secs.max(0.0) as u32;
                        let m = total / 60;
                        let s = total % 60;
                        format!("{m:02}:{s:02}")
                    };

                    let time_label = if player_dur > 0.0 {
                        format!("{} / {}", format_time(player_pos), format_time(player_dur))
                    } else {
                        format_time(player_pos)
                    };
                    ui.label(egui::RichText::new(time_label).monospace());

                    if player_dur > 0.0 {
                        let mut seek_pos = player_pos.clamp(0.0, player_dur);
                        let slider = ui.add(
                            egui::Slider::new(&mut seek_pos, 0.0..=player_dur).show_value(false),
                        );
                        if slider.drag_released() || (slider.changed() && !slider.dragged()) {
                            let _ = self.audio_tx.send(PlayerCommand::Seek(
                                std::time::Duration::from_secs_f32(seek_pos),
                            ));
                        }
                    }

                    ui.add_space(15.0);
                    ui.label("🔊");
                    let mut vol = self.volume;
                    if ui
                        .add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        self.volume = vol;
                        self.muted = false;
                        let _ = self.audio_tx.send(PlayerCommand::SetVolume(vol));
                    }
                    let mute_text = if self.muted {
                        "🔇 Unmute"
                    } else {
                        "🔈 Mute"
                    };
                    if ui.button(mute_text).clicked() {
                        self.muted = !self.muted;
                        let target_vol = if self.muted { 0.0 } else { self.volume };
                        let _ = self.audio_tx.send(PlayerCommand::SetVolume(target_vol));
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if !matches!(current_view, View::Login) {
                ui.horizontal(|ui| {
                    ui.label("🔍 Search YouTube:");
                    let response = ui.text_edit_singleline(&mut self.search_input);
                    if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        || ui.button("Search").clicked()
                    {
                        let q = self.search_input.trim().to_string();
                        if !q.is_empty() {
                            action = PendingAction::Search { query: q };
                        }
                    }
                });
                ui.add_space(5.0);

                // Navigation Tabs
                ui.horizontal(|ui| {
                    let on_subs = matches!(
                        current_view,
                        View::Subscriptions | View::ChannelVideos { .. }
                    );
                    if ui.selectable_label(on_subs, "📺 Subscriptions").clicked() {
                        action = PendingAction::GoToSubscriptions;
                    }
                    ui.add_space(10.0);
                    let on_new = matches!(current_view, View::NewVideos);
                    if ui.selectable_label(on_new, "🆕 New Videos").clicked() {
                        action = PendingAction::GoToNewVideos;
                    }
                    ui.add_space(10.0);
                    let on_playlists = matches!(
                        current_view,
                        View::Playlists { .. } | View::PlaylistVideos { .. }
                    );
                    if ui.selectable_label(on_playlists, "📂 Playlists").clicked() {
                        action = PendingAction::LoadPlaylists;
                    }
                    ui.add_space(10.0);
                    let on_settings = matches!(current_view, View::Settings);
                    if ui.selectable_label(on_settings, "⚙️ Settings").clicked() {
                        action = PendingAction::GoToSettings;
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

                // Toast notification banner
                let toast_data = {
                    let mut s = lock_state(&self.state);
                    if let Some((msg, time, is_err)) = &s.toast {
                        if time.elapsed().as_secs() < 5 {
                            Some((msg.clone(), *is_err))
                        } else {
                            s.toast = None;
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some((msg, is_err)) = toast_data {
                    ui.horizontal(|ui| {
                        let (bg, fg) = if is_err {
                            (
                                egui::Color32::from_rgb(85, 25, 25),
                                egui::Color32::from_rgb(255, 180, 180),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(25, 75, 45),
                                egui::Color32::from_rgb(180, 255, 200),
                            )
                        };
                        egui::Frame::none()
                            .fill(bg)
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(&msg).color(fg).small());
                            });
                    });
                    ui.add_space(3.0);
                }
            }

            match current_view {
                View::Login => {
                    let s_lock = lock_state(&self.state);
                    if let Some(act) = render_login_view(
                        ui,
                        &mut self.client_id_input,
                        &mut self.client_secret_input,
                        &s_lock,
                    ) {
                        action = act;
                    }
                }
                View::Subscriptions => {
                    let subs = {
                        let s = lock_state(&self.state);
                        s.subscriptions.clone()
                    };
                    if let Some(act) = render_subscriptions_view(
                        &self.state,
                        &self.http_client,
                        ui,
                        ctx,
                        &subs,
                        &mut self.subscription_filter,
                    ) {
                        action = act;
                    }
                }
                View::NewVideos => {
                    let (vids, cleared_count) = {
                        let s = lock_state(&self.state);
                        (s.new_videos.clone(), s.cleared_video_ids.len())
                    };
                    if let Some(act) = render_new_videos_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        &vids,
                        cleared_count,
                    ) {
                        action = act;
                    }
                }
                View::ChannelVideos {
                    channel_id,
                    channel_title,
                    channel_description,
                    videos,
                } => {
                    let subs = {
                        let s = lock_state(&self.state);
                        s.subscriptions.clone()
                    };
                    if let Some(act) = render_channel_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        &channel_id,
                        &channel_title,
                        &channel_description,
                        &videos,
                        &subs,
                    ) {
                        action = act;
                    }
                }
                View::SearchResults { query: _, videos } => {
                    if let Some(act) = render_search_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        &mut self.search_input,
                        &videos,
                    ) {
                        action = act;
                    }
                }
                View::Playlists { playlists } => {
                    if let Some(act) = render_playlists_view(
                        &self.state,
                        &self.http_client,
                        ui,
                        ctx,
                        &playlists,
                        &mut self.playlist_title_input,
                        &mut self.playlist_desc_input,
                    ) {
                        action = act;
                    }
                }
                View::PlaylistVideos {
                    playlist_id,
                    playlist_title,
                    videos,
                } => {
                    if let Some(act) = render_playlist_videos_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        &playlist_id,
                        &playlist_title,
                        &videos,
                    ) {
                        action = act;
                    }
                }
                View::VideoDetails {
                    video,
                    details,
                    comments,
                } => {
                    let (p_state, pls, status) = {
                        let s = lock_state(&self.state);
                        (
                            s.player_state.clone(),
                            s.playlists.clone(),
                            s.playlist_action_status.clone(),
                        )
                    };
                    if let Some(act) = render_details_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        &video,
                        &details,
                        &comments,
                        &p_state,
                        &pls,
                        &status,
                        &mut self.comment_input,
                    ) {
                        action = act;
                    }
                }
                View::Settings => {
                    let s = lock_state(&self.state);
                    if let Some(act) = render_settings_view(ui, &mut self.settings_form, &s) {
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
            PendingAction::SpawnDefaultLogin => {
                spawn_default_login(self.state.clone(), ctx.clone());
            }
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
                let cleared_path = s_lock.cleared_videos_path.clone();
                let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
                s_lock.new_videos = Some(Ok(Vec::new()));
            }
            PendingAction::DismissNewVideo { video_id } => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.cleared_video_ids.insert(video_id.clone());
                let cleared_path = s_lock.cleared_videos_path.clone();
                let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
                if let Some(Ok(ref mut vids)) = s_lock.new_videos {
                    vids.retain(|v| v.id != video_id);
                }
            }
            PendingAction::ResetClearedVideos => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.cleared_video_ids.clear();
                    let cleared_path = s_lock.cleared_videos_path.clone();
                    let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
                    s_lock.new_videos = None;
                }
                spawn_fetch_new_videos(self.state.clone(), ctx.clone());
            }
            PendingAction::LoadChannel {
                id,
                title,
                description,
            } => {
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
            PendingAction::RetryVideos {
                id,
                title,
                description,
            } => {
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
                let log_file = self.state.lock().unwrap().log_file.clone();
                if is_mp3 {
                    append_to_log(&log_file, "INFO", &format!("Playing local audio: {path:?}"));
                    let _ = self.audio_tx.send(PlayerCommand::Play(path, title));
                } else {
                    append_to_log(&log_file, "INFO", &format!("Opening local video: {path:?}"));
                    if let Err(e) =
                        launch_external_player_with_log(path.as_os_str(), Some(&log_file))
                    {
                        append_to_log(
                            &log_file,
                            "WARN",
                            &format!("{e} Falling back to default file opener."),
                        );
                        let _ = open::that(path);
                    }
                }
            }
            PendingAction::StreamVideo { video_id } => {
                let url = format!("https://www.youtube.com/watch?v={video_id}");
                let log_file = self.state.lock().unwrap().log_file.clone();
                append_to_log(&log_file, "INFO", &format!("Video clicked: {url}"));

                // Try to open the stream in MPV or VLC first
                if let Err(e) =
                    launch_external_player_with_log(std::ffi::OsStr::new(&url), Some(&log_file))
                {
                    append_to_log(
                        &log_file,
                        "WARN",
                        &format!("{e} Falling back to default browser."),
                    );
                    let mut s = lock_state(&self.state);
                    s.set_toast("Media player not found; opening in browser", true);
                    let _ = open::that(url);
                } else {
                    let mut s = lock_state(&self.state);
                    s.set_toast("Opening stream in external media player...", false);
                }
            }
            PendingAction::OpenInBrowser { url } => {
                let log_file = self.state.lock().unwrap().log_file.clone();
                append_to_log(&log_file, "INFO", &format!("Opening in web browser: {url}"));
                let mut s = lock_state(&self.state);
                s.set_toast("Opening in web browser...", false);
                let _ = open::that(url);
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
                    s_lock.navigate_clear_history(View::Playlists {
                        playlists: cached.clone(),
                    });
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
                        details: None,
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
                        details: None,
                        comments: None,
                    };
                    s_lock.playlists.clone()
                };
                spawn_fetch_comments(self.state.clone(), ctx.clone(), video);
                if cache.is_none() {
                    spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::PostComment { video_id, text } => {
                spawn_post_comment(self.state.clone(), ctx.clone(), video_id, text);
            }
            PendingAction::CreatePlaylist { title, description } => {
                spawn_create_playlist(self.state.clone(), ctx.clone(), title, description);
            }
            PendingAction::DeletePlaylist { playlist_id } => {
                spawn_delete_playlist(self.state.clone(), ctx.clone(), playlist_id);
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
            PendingAction::AddToPlaylist {
                playlist_id,
                playlist_title,
                video_id,
            } => {
                spawn_add_to_playlist(
                    self.state.clone(),
                    ctx.clone(),
                    playlist_id,
                    playlist_title,
                    video_id,
                );
            }
            PendingAction::GoToSettings => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.navigate_clear_history(View::Settings);
            }
            PendingAction::SaveSettings {
                player_path,
                downloads_dir,
                cookies_from_browser,
            } => {
                let mut config = youtube_client_lib::load_config();
                config.player_path = player_path;
                config.downloads_dir = downloads_dir.clone();
                config.cookies_from_browser = cookies_from_browser;

                let _ = youtube_client_lib::save_config(&config);

                let mut s_lock = self.state.lock().unwrap();
                if let Some(ref d) = downloads_dir {
                    s_lock.downloads_dir = PathBuf::from(d);
                }
                s_lock.set_toast("Settings saved successfully!", false);
            }
            PendingAction::SignOut => {
                let token_path = youtube_client_lib::resolve_token_cache_path();
                if token_path.exists() {
                    let _ = std::fs::remove_file(&token_path);
                }
                let mut s_lock = self.state.lock().unwrap();
                s_lock.subscriptions = None;
                s_lock.new_videos = None;
                s_lock.playlists = None;
                s_lock.current_view = View::Login;
                s_lock.view_history.clear();
                s_lock.set_toast("Signed out successfully.", false);
            }
            PendingAction::GoToAbout => {
                let mut s_lock = self.state.lock().unwrap();
                s_lock.navigate_clear_history(View::About);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut cli_log_file = None;
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--log-file" || args[i] == "-l") && i + 1 < args.len() {
            cli_log_file = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else if let Some(stripped) = args[i].strip_prefix("--log-file=") {
            cli_log_file = Some(PathBuf::from(stripped));
            i += 1;
        } else {
            i += 1;
        }
    }
    let log_file = youtube_client_lib::resolve_log_file_path(cli_log_file.as_deref());

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let config = youtube_client_lib::load_config();

    let (width, height) = match config.window_size {
        Some([w, h]) if w >= 400.0 && h >= 400.0 => (w, h),
        _ => (700.0, 700.0),
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([width, height])
        .with_min_inner_size([400.0, 400.0]);

    if let Some([x, y]) = config.window_pos {
        if x > -10000.0 && y >= 0.0 && x < 50000.0 && y < 50000.0 {
            viewport = viewport.with_position([x, y]);
        }
    }

    if config.window_maximized == Some(true) {
        viewport = viewport.with_maximized(true);
    }

    if let Ok(img) = image::load_from_memory(include_bytes!("../../assets/icon.png")) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        viewport = viewport.with_icon(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        });
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let log_file_clone = log_file.clone();
    eframe::run_native(
        "YouTube Premium Subscriptions Client",
        native_options,
        Box::new(move |cc| Box::new(YoutubeGuiApp::new(cc, log_file_clone))),
    )
}
