use crate::types::{AppState, PendingAction};
use eframe::egui;

pub fn render_login_view(
    ui: &mut egui::Ui,
    client_id_input: &mut String,
    client_secret_input: &mut String,
    state: &AppState,
) -> Option<PendingAction> {
    let mut action = None;

    ui.vertical_centered(|ui| {
        ui.add_space(25.0);
        ui.heading(
            egui::RichText::new("🔐 YouTube Sign-In")
                .size(24.0)
                .strong()
                .color(egui::Color32::from_rgb(255, 60, 60)),
        );
        ui.add_space(8.0);
        ui.label("Sign in with your Google account to access your subscriptions, playlists, and downloads.");
        ui.add_space(20.0);
    });

    ui.group(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            if state.logging_in {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Opening browser for Google authorization... Please complete login in your browser.");
                });
            } else {
                let btn = egui::Button::new(
                    egui::RichText::new("🚀 Sign in with Google")
                        .size(15.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(204, 0, 0))
                .min_size(egui::vec2(220.0, 40.0));

                if ui.add(btn).clicked() {
                    action = Some(PendingAction::SpawnDefaultLogin);
                }

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("A browser window will open to approve YouTube access.")
                        .size(12.0)
                        .color(egui::Color32::GRAY),
                );
            }
            ui.add_space(10.0);
        });

        ui.separator();
        ui.add_space(8.0);

        // Collapsible Advanced Settings for custom Google Cloud credentials
        ui.collapsing("⚙️ Advanced: Use Custom Google Cloud Project", |ui| {
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new("By default, the client uses built-in desktop credentials. If you hit YouTube API daily quota limits or prefer your own Google Cloud project, enter your credentials below:")
                    .size(11.0)
                    .color(egui::Color32::LIGHT_GRAY),
            );
            ui.add_space(8.0);

            ui.label("Custom Client ID:");
            ui.text_edit_singleline(client_id_input);
            ui.add_space(6.0);

            ui.label("Custom Client Secret:");
            ui.text_edit_singleline(client_secret_input);
            ui.add_space(10.0);

            if !state.logging_in
                && ui.button("Save & Sign In with Custom Keys").clicked() {
                    let id = client_id_input.trim().to_string();
                    let secret = client_secret_input.trim().to_string();
                    if !id.is_empty() && !secret.is_empty() {
                        action = Some(PendingAction::SpawnLogin { id, secret });
                    }
                }
            ui.add_space(5.0);
        });

        let display_error = state.login_error.as_ref().or({
            if let Some(Err(err)) = &state.subscriptions {
                Some(err)
            } else {
                None
            }
        });

        if let Some(err) = display_error {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), format!("⚠️ Error: {err}"));
        }
    });

    action
}
