use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::egui;
use youtube_client_lib::{Subscription, YoutubeClient};
use youtube_client_lib::utils::{
    get_download_path, launch_external_player, load_string_set_from_file,
    save_string_set_to_file, scan_downloads_dir, DownloadStatus,
};




#[derive(Clone)]
struct Thumbnail {
    texture: Option<egui::TextureHandle>,
    loading: bool,
}

#[derive(Clone)]
enum View {
    Login,
    Subscriptions,
    NewVideos,
    ChannelVideos {
        channel_id: String,
        channel_title: String,
        channel_description: String,
        videos: Option<Result<Vec<youtube_client_lib::Video>, String>>,
    },
    SearchResults {
        query: String,
        videos: Option<Result<Vec<youtube_client_lib::Video>, String>>,
    },
    Playlists {
        playlists: Option<Result<Vec<youtube_client_lib::Playlist>, String>>,
    },
    PlaylistVideos {
        playlist_id: String,
        playlist_title: String,
        videos: Option<Result<Vec<youtube_client_lib::Video>, String>>,
    },
    VideoDetails {
        video: youtube_client_lib::Video,
        comments: Option<Result<Vec<youtube_client_lib::Comment>, String>>,
    },
}

#[derive(Clone, Debug)]
struct PlayerState {
    current_title: String,
    playing: bool,
}

enum PlayerCommand {
    Play(PathBuf, String),
    Pause,
    Resume,
    Stop,
}

struct AppState {
    subscriptions: Option<Result<Vec<Subscription>, String>>,
    new_videos: Option<Result<Vec<youtube_client_lib::Video>, String>>,
    cleared_video_ids: std::collections::HashSet<String>,
    thumbnails: HashMap<String, Thumbnail>,
    current_view: View,
    view_history: Vec<View>,
    logging_in: bool,
    login_error: Option<String>,
    downloads: HashMap<String, DownloadStatus>,
    player_state: PlayerState,
    playlists: Option<Result<Vec<youtube_client_lib::Playlist>, String>>,
    playlist_action_status: Option<Result<String, String>>,
    downloads_dir: PathBuf,
}

impl AppState {
    fn navigate_to(&mut self, new_view: View) {
        self.view_history.push(self.current_view.clone());
        self.current_view = new_view;
    }

    fn navigate_clear_history(&mut self, new_view: View) {
        self.view_history.clear();
        self.current_view = new_view;
    }

    fn go_back(&mut self) {
        if let Some(prev) = self.view_history.pop() {
            self.current_view = prev;
        } else {
            self.current_view = View::Subscriptions;
        }
    }
}


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
        let state_clone = state.clone();
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            let mut stream_opt: Option<rodio::MixerDeviceSink> = None;
            let mut sink_opt: Option<rodio::Player> = None;

            loop {
                let cmd_opt = audio_rx.recv_timeout(std::time::Duration::from_millis(200));
                match cmd_opt {
                    Ok(cmd) => {
                        match cmd {
                            PlayerCommand::Play(path, title) => {
                                if let Some(sink) = &sink_opt {
                                    sink.stop();
                                }
                                if stream_opt.is_none() {
                                    if let Ok(stream) = rodio::DeviceSinkBuilder::open_default_sink() {
                                        let sink = rodio::Player::connect_new(stream.mixer());
                                        stream_opt = Some(stream);
                                        sink_opt = Some(sink);
                                    }
                                }
                                if let Some(sink) = &sink_opt {
                                    if let Ok(file) = std::fs::File::open(&path) {
                                        if let Ok(source) = rodio::Decoder::new(std::io::BufReader::new(file)) {
                                            sink.append(source);
                                            sink.play();
                                            let mut s = state_clone.lock().unwrap();
                                            s.player_state.current_title = title;
                                            s.player_state.playing = true;
                                        }
                                    }
                                }
                            }
                            PlayerCommand::Pause => {
                                if let Some(sink) = &sink_opt {
                                    sink.pause();
                                    let mut s = state_clone.lock().unwrap();
                                    s.player_state.playing = false;
                                }
                            }
                            PlayerCommand::Resume => {
                                if let Some(sink) = &sink_opt {
                                    sink.play();
                                    let mut s = state_clone.lock().unwrap();
                                    s.player_state.playing = true;
                                }
                            }
                            PlayerCommand::Stop => {
                                if let Some(sink) = &sink_opt {
                                    sink.stop();
                                    let mut s = state_clone.lock().unwrap();
                                    s.player_state.playing = false;
                                    s.player_state.current_title = String::new();
                                }
                            }
                        }
                        ctx_clone.request_repaint();
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Check if track ended
                        if let Some(sink) = &sink_opt {
                            if sink.empty() {
                                let mut updated = false;
                                {
                                    let mut s = state_clone.lock().unwrap();
                                    if s.player_state.playing {
                                        s.player_state.playing = false;
                                        s.player_state.current_title = String::new();
                                        updated = true;
                                    }
                                }
                                if updated {
                                    ctx_clone.request_repaint();
                                }
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break; // Channel closed
                    }
                }
            }
        });

        // Initialize YoutubeClient and fetch subscriptions asynchronously
        Self::spawn_fetch_subscriptions(state.clone(), cc.egui_ctx.clone());

        Self {
            state,
            http_client,
            client_id_input,
            client_secret_input,
            search_input: String::new(),
            audio_tx,
        }
    }

    async fn get_client_async() -> Result<YoutubeClient, String> {
        let config = youtube_client_lib::load_config();

        if !config.is_valid() {
            return Err(format!(
                "Google Client Credentials are not configured.\n\n{}",
                youtube_client_lib::GOOGLE_SETUP_INSTRUCTIONS
            ));
        }

        let client_id = config.client_id.unwrap();
        let client_secret = config.client_secret.unwrap();

        let token_cache_path = PathBuf::from("tokencache.json");
        if !token_cache_path.exists() {
            return Err("Token cache (tokencache.json) is missing. Please run the CLI login flow first: `cargo run --bin youtube-client -- login`".to_string());
        }

        // Verify if the cache contains the full youtube or force-ssl scope to prevent GUI hanging
        let has_full_scope = youtube_client_lib::check_token_cache_scopes(
            &token_cache_path,
            &[
                "https://www.googleapis.com/auth/youtube",
                "https://www.googleapis.com/auth/youtube.force-ssl",
            ],
        );

        if !has_full_scope {
            return Err("Token cache does not have full write permissions.\n\nPlease log in again via the terminal:\n`cargo run --bin youtube-client -- login`".to_string());
        }

        // 2. Initialize OAuth client
        let client = YoutubeClient::new_oauth_with_scopes(
            &client_id,
            &client_secret,
            &token_cache_path,
            youtube_client_lib::YOUTUBE_SCOPES,
        ).await
        .map_err(|e| format!("Authentication failed: {}", e))?;

        Ok(client)
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
            Self::fetch_thumbnail(
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

    fn spawn_fetch_new_videos(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_new_videos",
            |client| async move {
                let subs = client.list_subscriptions(10).await
                    .map_err(|e| format!("Failed to fetch subscriptions for new videos feed: {}", e))?;
                
                let mut all_videos = Vec::new();
                for sub in subs.iter().take(5) {
                    if let Ok(vids) = client.list_videos(&sub.channel_id, 5).await {
                        all_videos.extend(vids);
                    }
                }
                all_videos.sort_by(|a, b| b.published_at.cmp(&a.published_at));
                Ok(all_videos)
            },
            |res, s, ctx| {
                match res {
                    Ok(mut vids) => {
                        vids.retain(|v| !s.cleared_video_ids.contains(&v.id));
                        s.new_videos = Some(Ok(vids));
                    }
                    Err(e) => {
                        s.new_videos = Some(Err(e));
                    }
                }
                ctx.request_repaint();
            },
        );
    }


    fn spawn_fetch_subscriptions(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_subscriptions",
            |client| async move {
                client.list_subscriptions(50).await
                    .map_err(|e| format!("Failed to fetch subscriptions: {}", e))
            },
            |res, s, ctx| {
                match res {
                    Ok(subs) => {
                        s.subscriptions = Some(Ok(subs));
                    }
                    Err(e) => {
                        s.subscriptions = Some(Err(e));
                        s.current_view = View::Login;
                    }
                }
                ctx.request_repaint();
            },
        );
    }

    fn spawn_login_and_auth(state: Arc<Mutex<AppState>>, ctx: egui::Context, id: String, secret: String) {
        {
            let mut s = state.lock().unwrap();
            s.logging_in = true;
            s.login_error = None;
        }
        let state_clone = state.clone();
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let res = async {
                #[derive(serde::Serialize)]
                struct ConfigSave {
                    client_id: String,
                    client_secret: String,
                }
                let config_data = ConfigSave {
                    client_id: id.clone(),
                    client_secret: secret.clone(),
                };
                let content = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
                std::fs::write("private_config.json", content).map_err(|e| e.to_string())?;

                let token_cache_path = PathBuf::from("tokencache.json");
                let client = YoutubeClient::new_oauth_with_scopes(
                    &id,
                    &secret,
                    &token_cache_path,
                    youtube_client_lib::YOUTUBE_SCOPES,
                ).await
                .map_err(|e| format!("OAuth initialization failed: {}", e))?;
                client.test_connection().await
                    .map_err(|e| format!("YouTube connection failed: {}", e))?;
                Ok(())
            }.await;

            let mut s = state_clone.lock().unwrap();
            s.logging_in = false;
            match res {
                Ok(_) => {
                    s.current_view = View::Subscriptions;
                    s.subscriptions = None;
                    drop(s);
                    Self::spawn_fetch_subscriptions(state_clone, ctx_clone);
                }
                Err(e) => {
                    s.login_error = Some(e);
                    ctx_clone.request_repaint();
                }
            }
        });
    }

    fn spawn_download(state: Arc<Mutex<AppState>>, ctx: egui::Context, video: youtube_client_lib::Video, is_audio: bool) {
        let video_id = video.id.clone();
        {
            let mut s = state.lock().unwrap();
            s.downloads.insert(video_id.clone(), DownloadStatus::Downloading { progress: "Starting...".to_string() });
        }
        let state_clone = state.clone();
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let res = async {
                let downloads_base = {
                    let s = state_clone.lock().unwrap();
                    s.downloads_dir.clone()
                };

                let output_path = get_download_path(&downloads_base, &video.channel_title, &video.title, &video.id, is_audio);

                if let Some(parent) = output_path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                }

                let client = Self::get_client_async().await.map_err(|e| e.to_string())?;
                
                let state_inner = state_clone.clone();
                let ctx_inner = ctx_clone.clone();
                let vid_id = video_id.clone();
                client.download_video(&video_id, &output_path, move |prog| {
                    if let Ok(mut s) = state_inner.lock() {
                        s.downloads.insert(vid_id.clone(), DownloadStatus::Downloading { progress: prog.to_string() });
                    }
                    ctx_inner.request_repaint();
                }).await.map_err(|e| e.to_string())?;
                
                Ok(output_path)
            }.await;

            let mut s = state_clone.lock().unwrap();
            match res {
                Ok(path) => {
                    s.downloads.insert(video_id, DownloadStatus::Finished(path));
                }
                Err(e) => {
                    s.downloads.insert(video_id, DownloadStatus::Failed(e));
                }
            }
            ctx_clone.request_repaint();
        });
    }

    fn fetch_videos(
        ctx: egui::Context,
        state: Arc<Mutex<AppState>>,
        channel_id: String,
        _channel_title: String,
    ) {
        let channel_id_clone = channel_id.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_videos",
            move |client| async move {
                client.list_videos(&channel_id, 20).await
                    .map_err(|e| format!("Failed to fetch videos: {}", e))
            },
            move |res, s, ctx| {
                if let View::ChannelVideos { channel_id: current_id, channel_title: current_title, channel_description: current_desc, videos: _ } = &s.current_view {
                    if current_id == &channel_id_clone {
                        s.current_view = View::ChannelVideos {
                            channel_id: channel_id_clone,
                            channel_title: current_title.clone(),
                            channel_description: current_desc.clone(),
                            videos: Some(res),
                        };
                    }
                }
                ctx.request_repaint();
            },
        );
    }

    fn fetch_search_results(
        ctx: egui::Context,
        state: Arc<Mutex<AppState>>,
        query: String,
    ) {
        let query_clone = query.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_search_results",
            move |client| async move {
                client.search_videos(&query, 20).await
                    .map_err(|e| format!("Failed to search videos: {}", e))
            },
            move |res, s, ctx| {
                if let View::SearchResults { query: current_q, videos: _ } = &s.current_view {
                    if current_q == &query_clone {
                        s.current_view = View::SearchResults {
                            query: query_clone,
                            videos: Some(res),
                        };
                    }
                }
                ctx.request_repaint();
            },
        );
    }

    fn spawn_fetch_playlists(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_playlists",
            |client| async move {
                client.list_playlists(50).await
                    .map_err(|e| format!("Failed to fetch playlists: {}", e))
            },
            |res, s, ctx| {
                s.playlists = Some(res.clone());
                if let View::Playlists { playlists: _ } = &s.current_view {
                    s.current_view = View::Playlists { playlists: Some(res) };
                }
                ctx.request_repaint();
            },
        );
    }

    fn fetch_playlist_videos(
        ctx: egui::Context,
        state: Arc<Mutex<AppState>>,
        playlist_id: String,
        _playlist_title: String,
    ) {
        let playlist_id_clone = playlist_id.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_playlist_videos",
            move |client| async move {
                client.list_playlist_videos(&playlist_id, 50).await
                    .map_err(|e| format!("Failed to fetch playlist videos: {}", e))
            },
            move |res, s, ctx| {
                if let View::PlaylistVideos { playlist_id: current_id, playlist_title: current_title, videos: _ } = &s.current_view {
                    if current_id == &playlist_id_clone {
                        s.current_view = View::PlaylistVideos {
                            playlist_id: playlist_id_clone,
                            playlist_title: current_title.clone(),
                            videos: Some(res),
                        };
                    }
                }
                ctx.request_repaint();
            },
        );
    }

    fn spawn_fetch_comments(state: Arc<Mutex<AppState>>, ctx: egui::Context, video: youtube_client_lib::Video) {
        let video_clone = video.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "fetch_comments",
            move |client| async move {
                client.fetch_comments(&video.id).await
                    .map_err(|e| format!("Failed to fetch comments: {}", e))
            },
            move |res, s, ctx| {
                if let View::VideoDetails { video: current_video, comments: _ } = &s.current_view {
                    if current_video.id == video_clone.id {
                        s.current_view = View::VideoDetails {
                            video: video_clone,
                            comments: Some(res),
                        };
                    }
                }
                ctx.request_repaint();
            },
        );
    }

    fn spawn_client_action<F, Fut, T>(
        state: Arc<Mutex<AppState>>,
        ctx: egui::Context,
        action_name: &'static str,
        f: F,
        on_complete: impl FnOnce(Result<T, String>, &mut AppState, &egui::Context) + Send + 'static,
    )
    where
        F: FnOnce(YoutubeClient) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
        T: Send + 'static,
    {
        let state_clone = state.clone();
        tokio::spawn(async move {
            let res = async {
                let client = Self::get_client_async().await?;
                f(client).await
            }.await;

            if let Err(e) = &res {
                println!("Error during {}: {}", action_name, e);
            }
            let mut s = state_clone.lock().unwrap();
            on_complete(res, &mut *s, &ctx);
        });
    }

    fn spawn_subscribe(state: Arc<Mutex<AppState>>, ctx: egui::Context, channel_id: String) {
        let state_clone = state.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "subscribe",
            move |client| async move {
                client.subscribe_to_channel(&channel_id).await
                    .map_err(|e| format!("Failed to subscribe: {}", e))
            },
            move |res, _, ctx| {
                if res.is_ok() {
                    Self::spawn_fetch_subscriptions(state_clone, ctx.clone());
                }
            },
        );
    }

    fn spawn_unsubscribe(state: Arc<Mutex<AppState>>, ctx: egui::Context, subscription_id: String) {
        let state_clone = state.clone();
        Self::spawn_client_action(
            state,
            ctx,
            "unsubscribe",
            move |client| async move {
                client.unsubscribe_from_channel(&subscription_id).await
                    .map_err(|e| format!("Failed to unsubscribe: {}", e))
            },
            move |res, _, ctx| {
                if res.is_ok() {
                    Self::spawn_fetch_subscriptions(state_clone, ctx.clone());
                }
            },
        );
    }

    fn spawn_rate_video(state: Arc<Mutex<AppState>>, ctx: egui::Context, video_id: String, rating: String) {
        Self::spawn_client_action(
            state,
            ctx,
            "rate_video",
            move |client| async move {
                client.rate_video(&video_id, &rating).await
                    .map_err(|e| format!("Failed to rate: {}", e))?;
                Ok((video_id, rating))
            },
            move |res, _, _| {
                if let Ok((vid, rat)) = res {
                    println!("Successfully rated video {} as {}", vid, rat);
                }
            },
        );
    }

    fn spawn_add_to_playlist(state: Arc<Mutex<AppState>>, ctx: egui::Context, playlist_id: String, playlist_title: String, video_id: String) {
        Self::spawn_client_action(
            state,
            ctx,
            "add_to_playlist",
            move |client| async move {
                client.add_to_playlist(&playlist_id, &video_id).await
                    .map_err(|e| format!("Failed to add to playlist: {}", e))?;
                Ok(format!("Added to '{}'", playlist_title))
            },
            move |res, s, ctx| {
                s.playlist_action_status = Some(res);
                ctx.request_repaint();
            },
        );
    }




    fn fetch_thumbnail(
        ctx: egui::Context,
        state: Arc<Mutex<AppState>>,
        http_client: reqwest::Client,
        channel_id: String,
        url: String,
    ) {
        tokio::spawn(async move {
            let success = async {
                let response = http_client.get(&url).send().await.ok()?;
                let bytes = response.bytes().await.ok()?;
                let img = image::load_from_memory(&bytes).ok()?;
                let size = [img.width() as _, img.height() as _];
                let rgba = img.to_rgba8();
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

                // Load texture back on the GUI main context
                let texture = ctx.load_texture(
                    format!("thumb_{}", channel_id),
                    color_image,
                    Default::default(),
                );

                let mut s = state.lock().unwrap();
                if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                    t.texture = Some(texture);
                    t.loading = false;
                }
                ctx.request_repaint();
                Some(())
            }.await;

            if success.is_none() {
                // Mark loading as failed/finished so we don't try again
                let mut s = state.lock().unwrap();
                if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                    t.loading = false;
                }
            }
        });
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
                Self::spawn_login_and_auth(self.state.clone(), ctx.clone(), id, secret);
            }
            PendingAction::RetrySubscriptions => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.subscriptions = None;
                }
                Self::spawn_fetch_subscriptions(self.state.clone(), ctx.clone());
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
                    Self::spawn_fetch_new_videos(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::LoadNewVideos => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.new_videos = None;
                    s_lock.current_view = View::NewVideos;
                }
                Self::spawn_fetch_new_videos(self.state.clone(), ctx.clone());
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
                Self::spawn_fetch_new_videos(self.state.clone(), ctx.clone());
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
                Self::fetch_videos(ctx.clone(), self.state.clone(), id, title);
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
                Self::fetch_videos(ctx.clone(), self.state.clone(), id, title);
            }
            PendingAction::SpawnDownload { video, is_audio } => {
                Self::spawn_download(self.state.clone(), ctx.clone(), video, is_audio);
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
                Self::fetch_search_results(ctx.clone(), self.state.clone(), query);
            }
            PendingAction::RetrySearch { query } => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.current_view = View::SearchResults {
                        query: query.clone(),
                        videos: None,
                    };
                }
                Self::fetch_search_results(ctx.clone(), self.state.clone(), query);
            }
            PendingAction::LoadPlaylists => {
                let cache = {
                    let mut s_lock = self.state.lock().unwrap();
                    let cached = s_lock.playlists.clone();
                    s_lock.navigate_clear_history(View::Playlists { playlists: cached.clone() });
                    cached
                };
                if cache.is_none() {
                    Self::spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::RetryPlaylists => {
                {
                    let mut s_lock = self.state.lock().unwrap();
                    s_lock.playlists = None;
                    s_lock.current_view = View::Playlists { playlists: None };
                }
                Self::spawn_fetch_playlists(self.state.clone(), ctx.clone());
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
                Self::fetch_playlist_videos(ctx.clone(), self.state.clone(), id, title);
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
                Self::fetch_playlist_videos(ctx.clone(), self.state.clone(), id, title);
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
                Self::spawn_fetch_comments(self.state.clone(), ctx.clone(), video);
                if cache.is_none() {
                    Self::spawn_fetch_playlists(self.state.clone(), ctx.clone());
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
                Self::spawn_fetch_comments(self.state.clone(), ctx.clone(), video);
                if cache.is_none() {
                    Self::spawn_fetch_playlists(self.state.clone(), ctx.clone());
                }
            }
            PendingAction::Subscribe { channel_id } => {
                Self::spawn_subscribe(self.state.clone(), ctx.clone(), channel_id);
            }
            PendingAction::Unsubscribe { subscription_id } => {
                Self::spawn_unsubscribe(self.state.clone(), ctx.clone(), subscription_id);
            }
            PendingAction::RateVideo { video_id, rating } => {
                Self::spawn_rate_video(self.state.clone(), ctx.clone(), video_id, rating);
            }
            PendingAction::AddToPlaylist { playlist_id, playlist_title, video_id } => {
                Self::spawn_add_to_playlist(self.state.clone(), ctx.clone(), playlist_id, playlist_title, video_id);
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

