#![windows_subsystem = "windows"]
#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::large_enum_variant
)]
//! # YouTube GUI Application Entry Point
//!
//! Initializes the eframe window, manages application lifecycle state, audio worker spawning,
//! keyboard accelerators, and event dispatching.

mod actions;
mod handlers;
mod player;
mod types;
mod views;

use actions::*;
use handlers::dispatch_pending_action;
use types::*;
use views::*;

use eframe::egui;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::{
    load_playback_positions_from_file, load_string_set_from_file, scan_downloads_dir,
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
    library_state: LibraryViewState,
    history_state: HistoryViewState,
    search_filter_state: SearchFilterState,
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
        if let Ok(s_lock) = self.state.lock() {
            s_lock.save_playback_positions();
        }

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
        let playback_positions_path =
            youtube_client_lib::resolve_app_data_path("playback_positions.json");
        let playback_positions = load_playback_positions_from_file(&playback_positions_path);
        let quota_path = youtube_client_lib::resolve_app_data_path("api_quota.json");
        let quota_tracker = Arc::new(youtube_client_lib::QuotaTracker::load_from_file_or_new(
            &quota_path,
        ));

        let state = Arc::new(Mutex::new(AppState {
            subscriptions: None,
            new_videos: None,
            cleared_video_ids,
            thumbnails: HashMap::new(),
            thumbnail_lru: VecDeque::new(),
            current_view: View::Subscriptions,
            view_history: Vec::new(),
            logging_in: false,
            login_error: None,
            downloads,
            player_state: PlayerState {
                current_title: String::new(),
                current_video_id: None,
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
            playback_positions_path,
            playback_positions,
            quota_tracker,
            quota_path,
            mock_backend: None,
            cached_backend: None,
            toast: None,
            sponsor_segments: HashMap::new(),
            recent_searches: VecDeque::new(),
            last_dismissed_video: None,
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
            library_state: LibraryViewState::default(),
            history_state: HistoryViewState::default(),
            search_filter_state: SearchFilterState::default(),
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

    fn refresh_action_for_view(&self, current_view: &View) -> PendingAction {
        match current_view {
            View::Subscriptions => PendingAction::RetrySubscriptions,
            View::NewVideos => PendingAction::LoadNewVideos,
            View::ChannelVideos {
                channel_id,
                channel_title,
                channel_description,
                ..
            } => PendingAction::RetryVideos {
                id: channel_id.clone(),
                title: channel_title.clone(),
                description: channel_description.clone(),
            },
            View::SearchResults { query, .. } => PendingAction::RetrySearch {
                query: query.clone(),
                filter: Some(self.search_filter_state.to_search_filter()),
            },
            View::Playlists { .. } => PendingAction::RetryPlaylists,
            View::PlaylistVideos {
                playlist_id,
                playlist_title,
                ..
            } => PendingAction::RetryPlaylistVideos {
                id: playlist_id.clone(),
                title: playlist_title.clone(),
            },
            View::VideoDetails { video, .. } => PendingAction::RetryVideoDetails {
                video: video.clone(),
            },
            _ => PendingAction::None,
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

        let (
            current_view,
            has_history,
            is_login,
            player_title,
            player_playing,
            player_pos,
            player_dur,
        ) = {
            let s = lock_state(&self.state);
            (
                s.current_view.clone(),
                !s.view_history.is_empty(),
                matches!(s.current_view, View::Login),
                s.player_state.current_title.clone(),
                s.player_state.playing,
                s.player_state.position_secs,
                s.player_state.duration_secs,
            )
        };
        let mut action = PendingAction::None;
        let top_search_id = egui::Id::new("global_top_search_input");
        let mut focus_top_search = false;

        // Global keyboard shortcuts & accelerators (Heuristic 7: Flexibility & Efficiency of Use)
        if !is_login {
            let wants_kb = ctx.wants_keyboard_input();
            ctx.input(|i| {
                // Ctrl+S saves settings when in Settings view (even if a text input has focus)
                if matches!(current_view, View::Settings)
                    && i.modifiers.command
                    && i.key_pressed(egui::Key::S)
                {
                    let player = if self.settings_form.player_path.trim().is_empty() {
                        None
                    } else {
                        Some(self.settings_form.player_path.trim().to_string())
                    };
                    let dir = if self.settings_form.downloads_dir.trim().is_empty() {
                        None
                    } else {
                        Some(self.settings_form.downloads_dir.trim().to_string())
                    };
                    let cookies = if self.settings_form.cookies_from_browser.trim().is_empty() {
                        None
                    } else {
                        Some(self.settings_form.cookies_from_browser.trim().to_string())
                    };
                    let res = if self.settings_form.preferred_resolution.trim().is_empty() {
                        None
                    } else {
                        Some(self.settings_form.preferred_resolution.trim().to_string())
                    };
                    let bitrate = if self.settings_form.preferred_audio_bitrate.trim().is_empty() {
                        None
                    } else {
                        Some(self.settings_form.preferred_audio_bitrate.trim().to_string())
                    };
                    action = PendingAction::SaveSettings {
                        player_path: player,
                        downloads_dir: dir,
                        cookies_from_browser: cookies,
                        preferred_resolution: res,
                        preferred_audio_bitrate: bitrate,
                        sponsorblock_enabled: self.settings_form.sponsorblock_enabled,
                        sponsorblock_categories: self
                            .settings_form
                            .sponsorblock_categories
                            .clone(),
                    };
                }

                // Ctrl+F focuses top search bar
                if i.modifiers.command && i.key_pressed(egui::Key::F) {
                    focus_top_search = true;
                }

                // F5 or Ctrl+R refreshes current view
                if i.key_pressed(egui::Key::F5)
                    || (i.modifiers.command && i.key_pressed(egui::Key::R))
                {
                    let ref_act = self.refresh_action_for_view(&current_view);
                    if !matches!(ref_act, PendingAction::None) {
                        action = ref_act;
                    }
                }

                // Alt+Left or Escape (when not typing in a text field) navigates Back
                if (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
                    || (!wants_kb && i.key_pressed(egui::Key::Escape))
                {
                    if has_history
                        || matches!(
                            current_view,
                            View::ChannelVideos { .. }
                                | View::PlaylistVideos { .. }
                                | View::VideoDetails { .. }
                        )
                    {
                        action = PendingAction::GoBack;
                    }
                }

                if !wants_kb {
                    // Slash (/) focuses search bar
                    if i.key_pressed(egui::Key::Slash) {
                        focus_top_search = true;
                    }

                    // Space toggles pause/resume when audio is loaded
                    if i.key_pressed(egui::Key::Space) && !player_title.is_empty() {
                        if player_playing {
                            let _ = self.audio_tx.send(PlayerCommand::Pause);
                        } else {
                            let _ = self.audio_tx.send(PlayerCommand::Resume);
                        }
                    }

                    // Alt+1..7 switches top navigation tabs
                    if i.modifiers.alt {
                        if i.key_pressed(egui::Key::Num1) {
                            action = PendingAction::GoToSubscriptions;
                        } else if i.key_pressed(egui::Key::Num2) {
                            action = PendingAction::GoToNewVideos;
                        } else if i.key_pressed(egui::Key::Num3) {
                            action = PendingAction::LoadPlaylists;
                        } else if i.key_pressed(egui::Key::Num4) {
                            action = PendingAction::GoToLibrary;
                        } else if i.key_pressed(egui::Key::Num5) {
                            action = PendingAction::GoToHistory;
                        } else if i.key_pressed(egui::Key::Num6) {
                            action = PendingAction::GoToSettings;
                        } else if i.key_pressed(egui::Key::Num7) {
                            action = PendingAction::GoToAbout;
                        }
                    }
                }
            });
        }

        if !is_login && !player_title.is_empty() {
            if player_playing {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }

            egui::TopBottomPanel::bottom("audio_player").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎵 Playing:").strong());
                    ui.label(&player_title);

                    ui.add_space(15.0);

                    if ui
                        .button("⏪ 10s")
                        .on_hover_text("Rewind 10 seconds")
                        .clicked()
                    {
                        let _ = self.audio_tx.send(PlayerCommand::Skip(-10));
                    }

                    if player_playing {
                        if ui
                            .button("⏸ Pause")
                            .on_hover_text("Pause audio playback (Space)")
                            .clicked()
                        {
                            let _ = self.audio_tx.send(PlayerCommand::Pause);
                        }
                    } else if ui
                        .button("▶ Resume")
                        .on_hover_text("Resume audio playback (Space)")
                        .clicked()
                    {
                        let _ = self.audio_tx.send(PlayerCommand::Resume);
                    }

                    if ui
                        .button("⏩ 10s")
                        .on_hover_text("Fast-forward 10 seconds")
                        .clicked()
                    {
                        let _ = self.audio_tx.send(PlayerCommand::Skip(10));
                    }

                    if ui
                        .button("⏹ Stop")
                        .on_hover_text("Stop audio playback")
                        .clicked()
                    {
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
                        .on_hover_text(format!("Volume: {:.0}%", vol * 100.0))
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
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_input)
                            .id(top_search_id)
                            .hint_text("Search videos or channels (Ctrl+F / /)...")
                            .desired_width(260.0),
                    );
                    if focus_top_search {
                        response.request_focus();
                    }
                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        response.surrender_focus();
                    }
                    if !self.search_input.is_empty()
                        && ui
                            .button("✖")
                            .on_hover_text("Clear search box")
                            .clicked()
                    {
                        self.search_input.clear();
                    }
                    let can_search = !self.search_input.trim().is_empty();
                    let enter_pressed = response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && can_search;
                    let search_clicked = ui
                        .add_enabled(can_search, egui::Button::new("Search"))
                        .on_hover_text("Search YouTube (Enter)")
                        .on_disabled_hover_text("Enter a search term first")
                        .clicked();
                    if enter_pressed || search_clicked {
                        let q = self.search_input.trim().to_string();
                        action = PendingAction::Search {
                            query: q,
                            filter: Some(self.search_filter_state.to_search_filter()),
                        };
                    }
                });
                ui.add_space(5.0);

                // Navigation Tabs
                ui.horizontal_wrapped(|ui| {
                    let on_subs = matches!(
                        current_view,
                        View::Subscriptions | View::ChannelVideos { .. }
                    );
                    if ui
                        .selectable_label(on_subs, "📺 Subscriptions")
                        .on_hover_text("View your subscribed YouTube channels (Alt+1)")
                        .clicked()
                    {
                        action = PendingAction::GoToSubscriptions;
                    }
                    ui.add_space(8.0);
                    let on_new = matches!(current_view, View::NewVideos);
                    if ui
                        .selectable_label(on_new, "🆕 New Videos")
                        .on_hover_text("Latest uploads from your subscriptions (Alt+2)")
                        .clicked()
                    {
                        action = PendingAction::GoToNewVideos;
                    }
                    ui.add_space(8.0);
                    let on_playlists = matches!(
                        current_view,
                        View::Playlists { .. } | View::PlaylistVideos { .. }
                    );
                    if ui
                        .selectable_label(on_playlists, "📂 Playlists")
                        .on_hover_text("Manage your YouTube playlists (Alt+3)")
                        .clicked()
                    {
                        action = PendingAction::LoadPlaylists;
                    }
                    ui.add_space(8.0);
                    let on_library = matches!(current_view, View::Library);
                    if ui
                        .selectable_label(on_library, "📁 Library")
                        .on_hover_text("Browse downloaded offline MP4 & MP3 files (Alt+4)")
                        .clicked()
                    {
                        action = PendingAction::GoToLibrary;
                    }
                    ui.add_space(8.0);
                    let on_history = matches!(current_view, View::History);
                    if ui
                        .selectable_label(on_history, "⏱ History")
                        .on_hover_text("View & resume watched videos (Alt+5)")
                        .clicked()
                    {
                        action = PendingAction::GoToHistory;
                    }
                    ui.add_space(8.0);
                    let on_settings = matches!(current_view, View::Settings);
                    if ui
                        .selectable_label(on_settings, "⚙ Settings")
                        .on_hover_text("Configure player, downloads, SponsorBlock & quota (Alt+6)")
                        .clicked()
                    {
                        action = PendingAction::GoToSettings;
                    }
                    ui.add_space(8.0);
                    let on_about = matches!(current_view, View::About);
                    if ui
                        .selectable_label(on_about, "ℹ About & Help")
                        .on_hover_text("App info, keyboard shortcuts & troubleshooting (Alt+7)")
                        .clicked()
                    {
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
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&msg).color(fg).small());
                                    if ui
                                        .small_button("✖")
                                        .on_hover_text("Dismiss notification")
                                        .clicked()
                                    {
                                        let mut s = lock_state(&self.state);
                                        s.toast = None;
                                    }
                                });
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
                    channel_details,
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
                        &channel_details,
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
                        &mut self.search_filter_state,
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
                    comments_next_page_token,
                    loading_more_comments,
                } => {
                    let (p_state, pls, status) = {
                        let s = lock_state(&self.state);
                        (
                            s.player_state.clone(),
                            s.playlists.clone(),
                            s.playlist_action_status.clone(),
                        )
                    };
                    let vctx = views::details_view::VideoDetailsContext {
                        video: &video,
                        details: &details,
                        comments: &comments,
                        comments_next_page_token: &comments_next_page_token,
                        loading_more_comments,
                        player_state: &p_state,
                        playlists: &pls,
                        playlist_action_status: &status,
                        comment_input: &mut self.comment_input,
                    };
                    if let Some(act) = render_details_view(
                        &self.state,
                        &self.http_client,
                        &self.audio_tx,
                        ui,
                        ctx,
                        vctx,
                    ) {
                        action = act;
                    }
                }
                View::Library => {
                    if let Some(act) = render_library_view(&self.state, ui, &mut self.library_state)
                    {
                        action = act;
                    }
                }
                View::History => {
                    if let Some(act) = render_history_view(&self.state, ui, &mut self.history_state)
                    {
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

        dispatch_pending_action(action, &self.state, &self.audio_tx, ctx);
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

    if let Ok(img) = image::load_from_memory(include_bytes!("../assets/icon.png")) {
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
