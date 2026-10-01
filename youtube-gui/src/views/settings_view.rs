use crate::types::{AppState, PendingAction};
use eframe::egui;

pub struct SettingsFormState {
    pub player_path: String,
    pub downloads_dir: String,
    pub cookies_from_browser: String,
    pub cookies_file: String,
}

impl Default for SettingsFormState {
    fn default() -> Self {
        let config = youtube_client_lib::load_config();
        Self {
            player_path: config.player_path.unwrap_or_default(),
            downloads_dir: config
                .downloads_dir
                .unwrap_or_else(|| "downloads".to_string()),
            cookies_from_browser: config.cookies_from_browser.unwrap_or_default(),
            cookies_file: config.cookies_file.unwrap_or_default(),
        }
    }
}

pub fn render_settings_view(
    ui: &mut egui::Ui,
    form_state: &mut SettingsFormState,
    state: &AppState,
) -> Option<PendingAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("⚙️ Settings & Configuration")
                .size(22.0)
                .strong(),
        );
    });
    ui.add_space(15.0);

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("🎬 Video Playback").strong().size(15.0));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Select or specify the external media player used for high-definition video streaming.",
                )
                .size(12.0)
                .color(egui::Color32::LIGHT_GRAY),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Preferred Player:");
                if ui
                    .selectable_label(form_state.player_path.is_empty(), "Auto (MPV / VLC / Browser)")
                    .clicked()
                {
                    form_state.player_path.clear();
                }
                if ui
                    .selectable_label(form_state.player_path == "mpv", "MPV")
                    .clicked()
                {
                    form_state.player_path = "mpv".to_string();
                }
                if ui
                    .selectable_label(form_state.player_path == "vlc", "VLC")
                    .clicked()
                {
                    form_state.player_path = "vlc".to_string();
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Custom Player Path:");
                ui.text_edit_singleline(&mut form_state.player_path);
            });
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("📥 Media Downloads")
                    .strong()
                    .size(15.0),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Directory where downloaded videos and extracted MP3 audio tracks are saved.",
                )
                .size(12.0)
                .color(egui::Color32::LIGHT_GRAY),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Downloads Directory:");
                ui.text_edit_singleline(&mut form_state.downloads_dir);
            });
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("🍪 YouTube Cookie Authentication").strong().size(15.0));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Optional: Pass cookies to yt-dlp to access member-only videos or bypass bot verification.",
                )
                .size(12.0)
                .color(egui::Color32::LIGHT_GRAY),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Browser:");
                let browsers = ["", "chrome", "firefox", "edge", "brave"];
                for b in browsers {
                    let label = if b.is_empty() { "None" } else { b };
                    if ui
                        .selectable_label(form_state.cookies_from_browser == b, label)
                        .clicked()
                    {
                        form_state.cookies_from_browser = b.to_string();
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Cookies.txt File Path:");
                ui.text_edit_singleline(&mut form_state.cookies_file);
            });
        });
    });

    ui.add_space(15.0);

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("🔐 Account & Storage")
                    .strong()
                    .size(15.0),
            );
            ui.add_space(8.0);

            let token_path = youtube_client_lib::resolve_token_cache_path();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Token Cache:").strong());
                ui.label(
                    egui::RichText::new(token_path.display().to_string())
                        .color(egui::Color32::LIGHT_GRAY),
                );
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Client Log File:").strong());
                ui.label(
                    egui::RichText::new(state.log_file.display().to_string())
                        .color(egui::Color32::LIGHT_GRAY),
                );
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Cleared Videos State:").strong());
                ui.label(
                    egui::RichText::new(state.cleared_videos_path.display().to_string())
                        .color(egui::Color32::LIGHT_GRAY),
                );
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button(
                        egui::RichText::new("🚪 Sign Out & Disconnect Account")
                            .color(egui::Color32::from_rgb(255, 120, 120)),
                    )
                    .clicked()
                {
                    action = Some(PendingAction::SignOut);
                }
            });
        });
    });

    ui.add_space(20.0);

    ui.horizontal(|ui| {
        if ui
            .button(egui::RichText::new("💾 Save Settings").size(15.0).strong())
            .clicked()
        {
            let player = if form_state.player_path.trim().is_empty() {
                None
            } else {
                Some(form_state.player_path.trim().to_string())
            };
            let dir = if form_state.downloads_dir.trim().is_empty() {
                None
            } else {
                Some(form_state.downloads_dir.trim().to_string())
            };
            let cookies = if form_state.cookies_from_browser.trim().is_empty() {
                None
            } else {
                Some(form_state.cookies_from_browser.trim().to_string())
            };

            action = Some(PendingAction::SaveSettings {
                player_path: player,
                downloads_dir: dir,
                cookies_from_browser: cookies,
            });
        }
    });

    action
}
