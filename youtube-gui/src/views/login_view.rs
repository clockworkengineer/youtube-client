use eframe::egui;
use crate::types::{AppState, PendingAction};

pub fn render_login_view(
    ui: &mut egui::Ui,
    client_id_input: &mut String,
    client_secret_input: &mut String,
    state: &AppState,
) -> Option<PendingAction> {
    let mut action = None;

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
            ui.text_edit_singleline(client_id_input);
            ui.add_space(10.0);

            ui.label("Google Client Secret:");
            ui.text_edit_singleline(client_secret_input);
            ui.add_space(15.0);

            if state.logging_in {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Attempting authentication & launching browser flow...");
                });
            } else {
                if ui.button("Save & Login").clicked() {
                    let id = client_id_input.trim().to_string();
                    let secret = client_secret_input.trim().to_string();
                    if !id.is_empty() && !secret.is_empty() {
                        action = Some(PendingAction::SpawnLogin { id, secret });
                    }
                }
            }

            let display_error = state.login_error.as_ref().or_else(|| {
                if let Some(Err(err)) = &state.subscriptions {
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

    action
}
