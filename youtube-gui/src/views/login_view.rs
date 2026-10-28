use crate::types::{AppState, PendingAction};
use crate::views::{help_badge, render_error_banner};
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
                    ui.label("Waiting for Google authorization in your browser...");
                    if ui
                        .button("✖ Cancel")
                        .on_hover_text("Cancel the current sign-in attempt")
                        .clicked()
                    {
                        action = Some(PendingAction::CancelLogin);
                    }
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

                if ui
                    .add(btn)
                    .on_hover_text("Opens your default web browser to authorize YouTube access")
                    .clicked()
                {
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
        ui.collapsing("⚙ Advanced: Use Custom Google Cloud Project", |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Enter your own Google Cloud OAuth 2.0 Desktop credentials to use a dedicated API quota:",
                    )
                    .size(11.0)
                    .color(egui::Color32::LIGHT_GRAY),
                );
                help_badge(
                    ui,
                    "In Google Cloud Console -> APIs & Services -> Credentials, create an OAuth 2.0 Client ID of type 'Desktop app' with YouTube Data API v3 enabled.",
                );
            });
            ui.add_space(8.0);

            ui.label("Custom Client ID:");
            ui.add(
                egui::TextEdit::singleline(client_id_input)
                    .hint_text("e.g. 123456789-abc.apps.googleusercontent.com")
                    .desired_width(360.0),
            );
            ui.add_space(6.0);

            ui.label("Custom Client Secret:");
            ui.add(
                egui::TextEdit::singleline(client_secret_input)
                    .password(true)
                    .hint_text("e.g. GOCSPX-...")
                    .desired_width(360.0),
            );
            ui.add_space(10.0);

            let can_submit = !state.logging_in
                && !client_id_input.trim().is_empty()
                && !client_secret_input.trim().is_empty();

            if ui
                .add_enabled(
                    can_submit,
                    egui::Button::new("Save & Sign In with Custom Keys"),
                )
                .on_disabled_hover_text("Enter both a Client ID and Client Secret first")
                .clicked()
            {
                let id = client_id_input.trim().to_string();
                let secret = client_secret_input.trim().to_string();
                action = Some(PendingAction::SpawnLogin { id, secret });
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
            let mut banner_action = PendingAction::None;
            render_error_banner(
                ui,
                "Sign-In or Connection Issue",
                err,
                PendingAction::SpawnDefaultLogin,
                &mut banner_action,
            );
            if !matches!(banner_action, PendingAction::None) {
                action = Some(banner_action);
            }
        }
    });

    action
}
