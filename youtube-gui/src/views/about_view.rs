use eframe::egui;

pub fn render_about_view(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.heading(
            egui::RichText::new("📺 YouTube Client Workspace")
                .size(24.0)
                .strong()
                .color(egui::Color32::from_rgb(255, 60, 60)),
        );
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Version 0.1.2 — Portable Native Desktop Client")
                .size(14.0)
                .color(egui::Color32::from_rgb(180, 180, 190)),
        );
        ui.add_space(20.0);
    });

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("A modular, high-performance native desktop client for YouTube built with Rust, egui, and eframe.")
                    .size(14.0),
            );
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(15.0);

            ui.label(egui::RichText::new("✨ Key Features:").strong().size(15.0));
            ui.add_space(8.0);
            ui.label("• 🔐 Desktop OAuth2 authentication with automatic token caching & scope validation");
            ui.label("• 📺 Concurrent subscriptions fetching & upload feeds");
            ui.label("• 🆕 Persistent New Videos Feed with clear/dismiss support across sessions");
            ui.label("• 🎵 Background audio playback worker utilizing Rodio");
            ui.label("• 📥 Media Downloading for video & audio streams");
            ui.label("• 📂 Playlist management and video comment browsing");
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(15.0);

            ui.horizontal(|ui| {
                if ui.button("☕ Support on Buy Me a Coffee").clicked() {
                    let _ = open::that("https://buymeacoffee.com/roberttizz1");
                }
                ui.add_space(15.0);
                if ui.button("🌐 Google Developer Console").clicked() {
                    let _ = open::that("https://console.cloud.google.com/");
                }
            });
        });
    });
}
