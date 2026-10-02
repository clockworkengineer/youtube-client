use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::actions::{spawn_default_login, spawn_login_and_auth};
use crate::types::{AppState, PendingAction, View};

/// Handle authentication and session actions (Single Responsibility Principle).
pub fn handle_auth_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    ctx: &egui::Context,
) -> bool {
    match action {
        PendingAction::SpawnDefaultLogin => {
            spawn_default_login(state.clone(), ctx.clone());
            true
        }
        PendingAction::SpawnLogin { id, secret } => {
            spawn_login_and_auth(state.clone(), ctx.clone(), id.clone(), secret.clone());
            true
        }
        PendingAction::SignOut => {
            let token_path = youtube_client_lib::resolve_token_cache_path();
            if token_path.exists() {
                let _ = std::fs::remove_file(&token_path);
            }
            let mut s_lock = state.lock().unwrap();
            s_lock.subscriptions = None;
            s_lock.new_videos = None;
            s_lock.playlists = None;
            s_lock.current_view = View::Login;
            s_lock.view_history.clear();
            s_lock.set_toast("Signed out successfully.", false);
            true
        }
        _ => false,
    }
}
