use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::types::{AppState, PendingAction};

/// Handle application settings persistence actions (Single Responsibility Principle).
pub fn handle_settings_action(action: &PendingAction, state: &Arc<Mutex<AppState>>) -> bool {
    match action {
        PendingAction::SaveSettings {
            player_path,
            downloads_dir,
            cookies_from_browser,
        } => {
            let mut config = youtube_client_lib::load_config();
            config.player_path = player_path.clone();
            config.downloads_dir = downloads_dir.clone();
            config.cookies_from_browser = cookies_from_browser.clone();

            let _ = youtube_client_lib::save_config(&config);

            let mut s_lock = state.lock().unwrap();
            if let Some(d) = downloads_dir {
                s_lock.downloads_dir = PathBuf::from(d);
            }
            s_lock.set_toast("Settings saved successfully!", false);
            true
        }
        _ => false,
    }
}
