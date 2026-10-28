use eframe::egui;

pub fn render_about_view(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(16.0);
                ui.heading(
                    egui::RichText::new("📺 YouTube Client Workspace")
                        .size(24.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 60, 60)),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Version {} — Portable Native Desktop Client",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(180, 180, 190)),
                );
                ui.add_space(16.0);
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "A fast, privacy-friendly native desktop client for YouTube built with Rust, egui, and eframe.",
                        )
                        .size(14.0),
                    );
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("✨ Key Features").strong().size(15.0));
                    ui.add_space(6.0);
                    ui.label("• 🔐 Desktop OAuth2 authentication with automatic token caching & scope validation");
                    ui.label("• 📺 Subscriptions feed with RSS fallback and channel statistics");
                    ui.label("• 🆕 Persistent New Videos Feed with dismiss & undo support");
                    ui.label("• 🎵 Background audio streaming & local playback powered by Rodio");
                    ui.label("• 📥 Video (MP4) and Audio (MP3) downloads with selectable resolution & bitrate");
                    ui.label("• 🛡 Crowd-sourced SponsorBlock segment skipping & Return YouTube Dislike metrics");
                    ui.label("• 📦 OPML, Google Takeout CSV, and NewPipe JSON subscription backup & migration");
                });
            });

            ui.add_space(12.0);

            // Keyboard Shortcuts Reference (Heuristic 7 & 10)
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("⌨ Keyboard Shortcuts & Accelerators")
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(8.0);
                    egui::Grid::new("shortcuts_grid")
                        .num_columns(2)
                        .spacing([24.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            let shortcuts = [
                                ("Space", "Pause / Resume active audio playback"),
                                ("Esc or Alt + Left", "Navigate back to previous screen"),
                                ("F5 or Ctrl + R", "Refresh the current view"),
                                ("Ctrl + F or /", "Focus the top search bar"),
                                ("Ctrl + S", "Save changes when in Settings"),
                                ("Alt + 1 .. 7", "Quickly switch between top navigation tabs"),
                            ];
                            for (keys, desc) in shortcuts {
                                ui.label(
                                    egui::RichText::new(keys)
                                        .monospace()
                                        .strong()
                                        .color(egui::Color32::from_rgb(130, 200, 255)),
                                );
                                ui.label(desc);
                                ui.end_row();
                            }
                        });
                });
            });

            ui.add_space(12.0);

            // Troubleshooting & Quick Help (Heuristic 9 & 10)
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("❓ Quick Help & Troubleshooting")
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(6.0);
                    ui.collapsing("How do I install yt-dlp, ffmpeg, and mpv?", |ui| {
                        ui.label("External media tools enable downloading and external HD streaming:");
                        ui.label(
                            egui::RichText::new("• Windows (winget): winget install yt-dlp.yt-dlp Gyan.FFmpeg shinchiro.mpv")
                                .monospace()
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("• macOS (Homebrew): brew install yt-dlp ffmpeg mpv")
                                .monospace()
                                .size(12.0),
                        );
                    });
                    ui.collapsing("What should I do if I hit the daily YouTube API quota?", |ui| {
                        ui.label("• Each YouTube search uses 100 units out of the 10,000 daily quota, while listing subscriptions or channel videos uses only 1 unit (and falls back to zero-quota RSS feeds when possible).");
                        ui.label("• You can supply your own Google Cloud OAuth Client ID and Secret in the Sign-In screen's Advanced section to use a dedicated 10,000 unit quota.");
                    });
                    ui.collapsing("Why did a download fail with 'Sign in to confirm you're not a bot'?", |ui| {
                        ui.label("• Open Settings -> YouTube Cookie Authentication and select your web browser (e.g., Chrome, Firefox, Edge, Brave) so yt-dlp can use your browser session cookies.");
                    });
                });
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button("☕ Support on Buy Me a Coffee").clicked() {
                    let _ = open::that("https://buymeacoffee.com/roberttizz1");
                }
                ui.add_space(15.0);
                if ui.button("🌐 Google Cloud Console").clicked() {
                    let _ = open::that("https://console.cloud.google.com/");
                }
            });
            ui.add_space(12.0);
        });
}
