use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::egui;
use youtube_client_lib::{YoutubeClient, Subscription};

#[derive(Clone)]
struct Thumbnail {
    texture: Option<egui::TextureHandle>,
    loading: bool,
}

#[derive(Clone)]
enum View {
    Subscriptions,
    ChannelVideos {
        channel_id: String,
        channel_title: String,
        videos: Option<Result<Vec<youtube_client_lib::Video>, String>>,
    },
}

struct AppState {
    subscriptions: Option<Result<Vec<Subscription>, String>>,
    thumbnails: HashMap<String, Thumbnail>,
    current_view: View,
}

struct YoutubeGuiApp {
    state: Arc<Mutex<AppState>>,
    http_client: reqwest::Client,
}

impl YoutubeGuiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize the styling to make it look premium
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 27, 30);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(33, 37, 43);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(41, 46, 54);
        cc.egui_ctx.set_visuals(visuals);

        let state = Arc::new(Mutex::new(AppState {
            subscriptions: None,
            thumbnails: HashMap::new(),
            current_view: View::Subscriptions,
        }));

        let http_client = reqwest::Client::new();

        // Initialize YoutubeClient and fetch subscriptions asynchronously
        let state_clone = state.clone();
        let ctx_clone = cc.egui_ctx.clone();
        tokio::spawn(async move {
            let res = Self::initialize_and_fetch_async().await;
            let mut s = state_clone.lock().unwrap();
            match res {
                Ok(subs) => {
                    s.subscriptions = Some(Ok(subs));
                }
                Err(e) => {
                    s.subscriptions = Some(Err(e));
                }
            }
            ctx_clone.request_repaint();
        });

        Self { state, http_client }
    }

    async fn get_client_async() -> Result<YoutubeClient, String> {
        #[derive(serde::Deserialize, Default)]
        struct Config {
            client_id: Option<String>,
            client_secret: Option<String>,
        }

        // 1. Locate credentials
        let private_config = PathBuf::from("private_config.json");
        let fallback_config = PathBuf::from("config.json");
        let token_cache_path = PathBuf::from("tokencache.json");

        let config_path = if private_config.exists() {
            private_config
        } else {
            fallback_config
        };

        if !config_path.exists() {
            return Err("Configuration file (private_config.json or config.json) is missing.".to_string());
        }

        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: Config = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let client_id = config.client_id.filter(|s| s != "ENTER_YOUR_CLIENT_ID_HERE" && !s.is_empty())
            .ok_or_else(|| "Google Client ID is not configured.".to_string())?;
        let client_secret = config.client_secret.filter(|s| s != "ENTER_YOUR_CLIENT_SECRET_HERE" && !s.is_empty())
            .ok_or_else(|| "Google Client Secret is not configured.".to_string())?;

        if !token_cache_path.exists() {
            return Err("Token cache (tokencache.json) is missing. Please run the CLI login flow first: `cargo run --bin youtube-client -- login`".to_string());
        }

        // 2. Initialize OAuth client
        let client = YoutubeClient::new_oauth(&client_id, &client_secret, &token_cache_path).await
            .map_err(|e| format!("Authentication failed: {}", e))?;

        Ok(client)
    }

    fn get_player_path() -> Option<String> {
        #[derive(serde::Deserialize, Default)]
        struct Config {
            player_path: Option<String>,
        }

        let private_config = PathBuf::from("private_config.json");
        let fallback_config = PathBuf::from("config.json");

        let config_path = if private_config.exists() {
            private_config
        } else {
            fallback_config
        };

        if let Ok(config_content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<Config>(&config_content) {
                return config.player_path.filter(|s| !s.is_empty() && s != "ENTER_PATH_TO_MEDIA_PLAYER_HERE");
            }
        }
        None
    }

    async fn initialize_and_fetch_async() -> Result<Vec<Subscription>, String> {
        let client = Self::get_client_async().await?;
        let subs = client.list_subscriptions(50).await
            .map_err(|e| format!("Failed to fetch subscriptions: {}", e))?;
        Ok(subs)
    }

    fn fetch_videos(
        ctx: egui::Context,
        state: Arc<Mutex<AppState>>,
        channel_id: String,
        _channel_title: String,
    ) {
        let state_clone = state.clone();
        tokio::spawn(async move {
            let res = async {
                let client = Self::get_client_async().await?;
                let videos = client.list_videos(&channel_id, 20).await
                    .map_err(|e| format!("Failed to fetch videos: {}", e))?;
                Ok(videos)
            }.await;

            let mut s = state_clone.lock().unwrap();
            if let View::ChannelVideos { channel_id: current_id, channel_title: current_title, videos: _ } = &s.current_view {
                if current_id == &channel_id {
                    s.current_view = View::ChannelVideos {
                        channel_id,
                        channel_title: current_title.clone(),
                        videos: Some(res),
                    };
                }
            }
            ctx.request_repaint();
        });
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

impl eframe::App for YoutubeGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (current_view, subscriptions) = {
            let s = self.state.lock().unwrap();
            (s.current_view.clone(), s.subscriptions.clone())
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            match current_view {
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
                                    {
                                        let mut s = self.state.lock().unwrap();
                                        s.subscriptions = None;
                                    }
                                    let state_clone = self.state.clone();
                                    let ctx_clone = ctx.clone();
                                    tokio::spawn(async move {
                                        let res = Self::initialize_and_fetch_async().await;
                                        let mut s = state_clone.lock().unwrap();
                                        match res {
                                            Ok(subs) => {
                                                s.subscriptions = Some(Ok(subs));
                                            }
                                            Err(e) => {
                                                s.subscriptions = Some(Err(e));
                                            }
                                        }
                                        ctx_clone.request_repaint();
                                    });
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
                                            // Start fetching thumbnail if not in the map
                                            let mut start_fetch = false;
                                            let texture = {
                                                let mut s = self.state.lock().unwrap();
                                                let thumbnail_entry = s.thumbnails.entry(sub.channel_id.clone()).or_insert_with(|| Thumbnail {
                                                    texture: None,
                                                    loading: false,
                                                 });

                                                if thumbnail_entry.texture.is_none() && !thumbnail_entry.loading && !sub.thumbnail_url.is_empty() {
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
                                                    sub.channel_id.clone(),
                                                    sub.thumbnail_url.clone(),
                                                );
                                            }

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
                                                {
                                                    let mut s = self.state.lock().unwrap();
                                                    s.current_view = View::ChannelVideos {
                                                        channel_id: sub.channel_id.clone(),
                                                        channel_title: sub.title.clone(),
                                                        videos: None,
                                                    };
                                                }
                                                Self::fetch_videos(
                                                    ctx.clone(),
                                                    self.state.clone(),
                                                    sub.channel_id.clone(),
                                                    sub.title.clone(),
                                                );
                                            }

                                            if response.hovered() {
                                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                            }

                                            ui.add_space(8.0);
                                        }
                                    });
                            }
                        }
                    }
                }
                View::ChannelVideos { channel_id, channel_title, videos } => {
                    // Header Area
                    ui.horizontal(|ui| {
                        if ui.button("⬅ Go Back").clicked() {
                            let mut s = self.state.lock().unwrap();
                            s.current_view = View::Subscriptions;
                        }
                        ui.add_space(15.0);
                        ui.heading(
                            egui::RichText::new(format!("Videos: {}", channel_title))
                                .size(20.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
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
                                    {
                                        let mut s = self.state.lock().unwrap();
                                        s.current_view = View::ChannelVideos {
                                            channel_id: channel_id.clone(),
                                            channel_title: channel_title.clone(),
                                            videos: None,
                                        };
                                    }
                                    Self::fetch_videos(
                                        ctx.clone(),
                                        self.state.clone(),
                                        channel_id.clone(),
                                        channel_title.clone(),
                                    );
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
                                            // Start fetching video thumbnail if not in the map
                                            let mut start_fetch = false;
                                            let texture = {
                                                let mut s = self.state.lock().unwrap();
                                                let thumbnail_entry = s.thumbnails.entry(video.id.clone()).or_insert_with(|| Thumbnail {
                                                    texture: None,
                                                    loading: false,
                                                });

                                                if thumbnail_entry.texture.is_none() && !thumbnail_entry.loading && !video.thumbnail_url.is_empty() {
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
                                                    video.id.clone(),
                                                    video.thumbnail_url.clone(),
                                                );
                                            }

                                            let response = ui.group(|ui| {
                                                ui.horizontal(|ui| {
                                                    // Thumbnail Render
                                                    if let Some(tex) = &texture {
                                                        ui.add(egui::Image::from_texture(tex).max_width(100.0).max_height(100.0));
                                                    } else {
                                                        // Placeholder thumbnail
                                                        let (rect, _response) = ui.allocate_exact_size(
                                                            egui::vec2(100.0, 100.0),
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
                                                            "🎬",
                                                            egui::FontId::proportional(40.0),
                                                            egui::Color32::LIGHT_GRAY,
                                                        );
                                                    }

                                                    ui.add_space(15.0);

                                                    // Text Info
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
                                            });

                                            let response = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                                            if response.clicked() {
                                                let url = format!("https://www.youtube.com/watch?v={}", video.id);
                                                println!("Video clicked: {}", url);
                                                
                                                // Try to open the stream in MPV or VLC first
                                                let mut players = Vec::new();
                                                let resolved_path = Self::get_player_path();
                                                println!("Resolved player path from config: {:?}", resolved_path);
                                                if let Some(user_player) = resolved_path {
                                                    players.push(user_player);
                                                }
                                                players.extend(vec![
                                                    "mpv".to_string(),
                                                    "vlc".to_string(),
                                                    "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe".to_string(),
                                                    "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe".to_string(),
                                                ]);
                                                
                                                let mut opened = false;
                                                for player in players {
                                                    print!("Trying player: {} ... ", player);
                                                    match std::process::Command::new(&player)
                                                        .arg(&url)
                                                        .spawn()
                                                    {
                                                        Ok(_) => {
                                                            println!("SUCCESS!");
                                                            opened = true;
                                                            break;
                                                        }
                                                        Err(e) => {
                                                            println!("FAILED ({})", e);
                                                        }
                                                    }
                                                }

                                                if !opened {
                                                    println!("No media players succeeded. Falling back to default browser.");
                                                    let _ = open::that(url);
                                                }
                                            }

                                            if response.hovered() {
                                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                            }

                                            ui.add_space(8.0);
                                        }
                                    });
                            }
                        }
                    }
                }
            }
        });
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
