use crate::types::{AppState, PendingAction};
use crate::views::help_badge;
use eframe::egui;

pub struct SettingsFormState {
    pub player_path: String,
    pub downloads_dir: String,
    pub cookies_from_browser: String,
    pub cookies_file: String,
    pub import_file_path: String,
    pub preferred_resolution: String,
    pub preferred_audio_bitrate: String,
    pub sponsorblock_enabled: bool,
    pub sponsorblock_categories: Vec<String>,
}

impl Default for SettingsFormState {
    fn default() -> Self {
        let config = youtube_client_lib::load_config();
        let default_cats = youtube_client_lib::sponsorblock::DEFAULT_CATEGORIES
            .iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            player_path: config.player_path.unwrap_or_default(),
            downloads_dir: config
                .downloads_dir
                .unwrap_or_else(|| "downloads".to_string()),
            cookies_from_browser: config.cookies_from_browser.unwrap_or_default(),
            cookies_file: config.cookies_file.unwrap_or_default(),
            import_file_path: String::new(),
            preferred_resolution: config
                .preferred_resolution
                .unwrap_or_else(|| "1080p".to_string()),
            preferred_audio_bitrate: config
                .preferred_audio_bitrate
                .unwrap_or_else(|| "320".to_string()),
            sponsorblock_enabled: config.sponsorblock_enabled.unwrap_or(true),
            sponsorblock_categories: config.sponsorblock_categories.unwrap_or(default_cats),
        }
    }
}

impl SettingsFormState {
    pub fn reset_to_defaults(&mut self) {
        self.player_path.clear();
        self.downloads_dir = "downloads".to_string();
        self.cookies_from_browser.clear();
        self.cookies_file.clear();
        self.preferred_resolution = "1080p".to_string();
        self.preferred_audio_bitrate = "320".to_string();
        self.sponsorblock_enabled = true;
        self.sponsorblock_categories = youtube_client_lib::sponsorblock::DEFAULT_CATEGORIES
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
}

pub fn render_settings_view(
    ui: &mut egui::Ui,
    form_state: &mut SettingsFormState,
    state: &AppState,
) -> Option<PendingAction> {
    let mut action = None;

    let confirm_signout_id = ui.make_persistent_id("confirm_signout_toggle");
    let mut confirm_signout = ui
        .data_mut(|d| d.get_temp::<bool>(confirm_signout_id))
        .unwrap_or(false);

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("⚙ Settings & Configuration")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(16.0);
        if ui
            .button(egui::RichText::new("💾 Save Settings").strong())
            .on_hover_text("Save configuration changes to disk (Ctrl+S)")
            .clicked()
        {
            action = Some(build_save_settings_action(form_state));
        }
        if ui
            .button("↺ Reset Defaults")
            .on_hover_text("Restore recommended default settings")
            .clicked()
        {
            form_state.reset_to_defaults();
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🎬 Video Playback").strong().size(15.0));
                        help_badge(
                            ui,
                            "MPV is recommended because it supports direct YouTube streaming, timestamp resume, and automatic SponsorBlock segment skipping.",
                        );
                    });
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
                            .selectable_label(
                                form_state.player_path.is_empty(),
                                "Auto (MPV / VLC / Browser)",
                            )
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
                        ui.add(
                            egui::TextEdit::singleline(&mut form_state.player_path)
                                .hint_text("Leave empty for automatic detection")
                                .desired_width(280.0),
                        );
                    });
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📥 Media Downloads & Quality Defaults")
                                .strong()
                                .size(15.0),
                        );
                        help_badge(
                            ui,
                            "Downloads are powered by yt-dlp and organized into Channel subfolders inside your Downloads Directory.",
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Directory and default quality constraints for downloaded MP4 videos and MP3 audio tracks.",
                        )
                        .size(12.0)
                        .color(egui::Color32::LIGHT_GRAY),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Downloads Directory:");
                        ui.add(
                            egui::TextEdit::singleline(&mut form_state.downloads_dir)
                                .desired_width(260.0),
                        );
                        if ui
                            .button("📂 Open")
                            .on_hover_text("Open downloads folder")
                            .clicked()
                        {
                            action = Some(PendingAction::OpenFolder {
                                path: state.downloads_dir.clone(),
                            });
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Default Video Resolution:");
                        let resolutions =
                            ["best", "2160p", "1440p", "1080p", "720p", "480p", "360p"];
                        for &res in &resolutions {
                            let label = match res {
                                "best" => "Best",
                                r => r,
                            };
                            if ui
                                .selectable_label(form_state.preferred_resolution == res, label)
                                .clicked()
                            {
                                form_state.preferred_resolution = res.to_string();
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Default Audio Bitrate:");
                        let bitrates = ["best", "320", "256", "192", "128"];
                        for &br in &bitrates {
                            let label = match br {
                                "best" => "Best",
                                "320" => "320 kbps",
                                "256" => "256 kbps",
                                "192" => "192 kbps",
                                "128" => "128 kbps",
                                _ => br,
                            };
                            if ui
                                .selectable_label(form_state.preferred_audio_bitrate == br, label)
                                .clicked()
                            {
                                form_state.preferred_audio_bitrate = br.to_string();
                            }
                        }
                    });
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🛡 SponsorBlock Integration")
                                .strong()
                                .size(15.0),
                        );
                        help_badge(
                            ui,
                            "Queries the public SponsorBlock API (sponsor.ajay.app) to skip sponsored segments, intros, and outros automatically.",
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Automatically skip sponsored segments, intros, outros, and reminders in MPV streams and highlight them on the timeline.",
                        )
                        .size(12.0)
                        .color(egui::Color32::LIGHT_GRAY),
                    );
                    ui.add_space(8.0);

                    ui.checkbox(
                        &mut form_state.sponsorblock_enabled,
                        "Enable SponsorBlock automatic skipping & segment markers",
                    );

                    if form_state.sponsorblock_enabled {
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Categories to skip:");
                            let available_categories = [
                                ("sponsor", "🟩 Sponsor"),
                                ("selfpromo", "🟨 Self-Promotion"),
                                ("interaction", "🟪 Interaction Reminder"),
                                ("intro", "🟦 Intro / Intermission"),
                                ("outro", "🟦 Endcards / Credits"),
                                ("preview", "🟦 Preview / Recap"),
                                ("music_offtopic", "🟧 Non-Music Section"),
                            ];
                            for (cat_id, label) in available_categories {
                                let mut is_checked = form_state
                                    .sponsorblock_categories
                                    .iter()
                                    .any(|c| c == cat_id);
                                if ui.checkbox(&mut is_checked, label).changed() {
                                    if is_checked {
                                        if !form_state
                                            .sponsorblock_categories
                                            .iter()
                                            .any(|c| c == cat_id)
                                        {
                                            form_state
                                                .sponsorblock_categories
                                                .push(cat_id.to_string());
                                        }
                                    } else {
                                        form_state
                                            .sponsorblock_categories
                                            .retain(|c| c != cat_id);
                                    }
                                }
                            }
                        });
                    }
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🍪 YouTube Cookie Authentication")
                                .strong()
                                .size(15.0),
                        );
                        help_badge(
                            ui,
                            "Passing browser cookies allows yt-dlp to download age-restricted or member-only streams and avoid 'Sign in to confirm you're not a bot' errors.",
                        );
                    });
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
                        ui.add(
                            egui::TextEdit::singleline(&mut form_state.cookies_file)
                                .hint_text("Optional Netscape cookies.txt path...")
                                .desired_width(280.0),
                        );
                    });
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📊 YouTube Data API Quota Budget")
                                .strong()
                                .size(15.0),
                        );
                        help_badge(
                            ui,
                            "Google Cloud grants 10,000 units/day by default. List/Read calls cost 1 unit, Write actions cost 50 units, and Searches cost 100 units.",
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Daily quota allocated by Google Cloud Console (tracked per UTC day).",
                        )
                        .size(12.0)
                        .color(egui::Color32::LIGHT_GRAY),
                    );
                    ui.add_space(8.0);

                    let quota = state.quota_tracker.status();
                    let pct = quota.percentage_used();
                    let gauge_color = if pct > 0.9 {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else if pct > 0.75 {
                        egui::Color32::from_rgb(255, 180, 50)
                    } else {
                        egui::Color32::from_rgb(80, 220, 120)
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Consumed Today ({} UTC):", quota.date))
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {} units ({:.1}%)",
                                quota.units_used,
                                quota.daily_limit,
                                pct * 100.0
                            ))
                            .color(gauge_color)
                            .strong(),
                        );
                    });

                    ui.add_space(4.0);
                    ui.add(
                        egui::ProgressBar::new(pct)
                            .fill(gauge_color)
                            .desired_width(260.0),
                    );

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Breakdown: {} searches (100u each) • {} list/read queries (1u each) • {} write actions (50u each)",
                            quota.search_count, quota.read_count, quota.write_count
                        ))
                        .size(11.0)
                        .color(egui::Color32::GRAY),
                    );
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("📦 Subscriptions Backup & Migration")
                                .strong()
                                .size(15.0),
                        );
                        help_badge(
                            ui,
                            "Supports OPML XML files, Google Takeout CSVs, and NewPipe JSON exports.",
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Export your subscriptions to standard OPML format or import from OPML, CSV, or NewPipe JSON.",
                        )
                        .size(12.0)
                        .color(egui::Color32::LIGHT_GRAY),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                egui::RichText::new("📤 Export Subscriptions to OPML")
                                    .color(egui::Color32::from_rgb(100, 200, 255))
                                    .strong(),
                            )
                            .on_hover_text(
                                "Save youtube_subscriptions.opml into your Downloads directory",
                            )
                            .clicked()
                        {
                            action = Some(PendingAction::ExportSubscriptionsOpml);
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Import File (OPML / CSV / JSON):");
                        ui.add(
                            egui::TextEdit::singleline(&mut form_state.import_file_path)
                                .hint_text("Path to subscriptions file...")
                                .desired_width(240.0),
                        );
                        let can_import = !form_state.import_file_path.trim().is_empty();
                        if ui
                            .add_enabled(
                                can_import,
                                egui::Button::new(
                                    egui::RichText::new("📥 Import")
                                        .color(egui::Color32::from_rgb(100, 255, 150))
                                        .strong(),
                                ),
                            )
                            .on_hover_text("Import channels from file into your Subscriptions list")
                            .on_disabled_hover_text("Enter a file path to import first")
                            .clicked()
                        {
                            let trimmed = form_state.import_file_path.trim();
                            action = Some(PendingAction::ImportSubscriptionsFile {
                                path: std::path::PathBuf::from(trimmed),
                            });
                        }
                    });
                });
            });

            ui.add_space(12.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("🔐 Account & Diagnostic Storage")
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if confirm_signout {
                            ui.label(
                                egui::RichText::new("Disconnect Google account?")
                                    .color(egui::Color32::from_rgb(255, 180, 100)),
                            );
                            if ui
                                .button(
                                    egui::RichText::new("✔ Yes, Sign Out")
                                        .color(egui::Color32::from_rgb(255, 100, 100))
                                        .strong(),
                                )
                                .clicked()
                            {
                                confirm_signout = false;
                                action = Some(PendingAction::SignOut);
                            }
                            if ui.button("Cancel").clicked() {
                                confirm_signout = false;
                            }
                        } else if ui
                            .button(
                                egui::RichText::new("🚪 Sign Out & Disconnect Account")
                                    .color(egui::Color32::from_rgb(255, 120, 120)),
                            )
                            .on_hover_text("Remove cached OAuth token and return to Sign-In screen")
                            .clicked()
                        {
                            confirm_signout = true;
                        }

                        if ui
                            .button("📂 Open Log Folder")
                            .on_hover_text("Open folder containing diagnostic logs and state files")
                            .clicked()
                        {
                            action = Some(PendingAction::OpenFolder {
                                path: state.log_file.clone(),
                            });
                        }
                    });

                    ui.add_space(6.0);
                    ui.collapsing("Diagnostic File Paths", |ui| {
                        let token_path = youtube_client_lib::resolve_token_cache_path();
                        ui.label(
                            egui::RichText::new(format!("Token Cache: {}", token_path.display()))
                                .monospace()
                                .size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Client Log File: {}",
                                state.log_file.display()
                            ))
                            .monospace()
                            .size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Cleared Videos State: {}",
                                state.cleared_videos_path.display()
                            ))
                            .monospace()
                            .size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Quota Tracking State: {}",
                                state.quota_path.display()
                            ))
                            .monospace()
                            .size(11.0),
                        );
                    });
                });
            });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("💾 Save Settings").size(15.0).strong())
                    .on_hover_text("Save configuration changes to disk (Ctrl+S)")
                    .clicked()
                {
                    action = Some(build_save_settings_action(form_state));
                }
            });
            ui.add_space(12.0);
        });

    ui.data_mut(|d| d.insert_temp(confirm_signout_id, confirm_signout));

    action
}

fn build_save_settings_action(form_state: &SettingsFormState) -> PendingAction {
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
    let res = if form_state.preferred_resolution.trim().is_empty() {
        None
    } else {
        Some(form_state.preferred_resolution.trim().to_string())
    };
    let bitrate = if form_state.preferred_audio_bitrate.trim().is_empty() {
        None
    } else {
        Some(form_state.preferred_audio_bitrate.trim().to_string())
    };

    PendingAction::SaveSettings {
        player_path: player,
        downloads_dir: dir,
        cookies_from_browser: cookies,
        preferred_resolution: res,
        preferred_audio_bitrate: bitrate,
        sponsorblock_enabled: form_state.sponsorblock_enabled,
        sponsorblock_categories: form_state.sponsorblock_categories.clone(),
    }
}
