use eframe::egui;
use std::sync::{Arc, Mutex};

use crate::actions::spawn_fetch_new_videos;
use crate::types::{AppState, PendingAction, View};

/// Handle GUI navigation and view routing actions (Single Responsibility Principle).
pub fn handle_navigation_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    ctx: &egui::Context,
) -> bool {
    match action {
        PendingAction::GoBack => {
            let mut s_lock = state.lock().unwrap();
            s_lock.go_back();
            true
        }
        PendingAction::GoToSubscriptions => {
            let mut s_lock = state.lock().unwrap();
            s_lock.navigate_clear_history(View::Subscriptions);
            true
        }
        PendingAction::GoToNewVideos => {
            let has_cache = {
                let mut s_lock = state.lock().unwrap();
                s_lock.navigate_clear_history(View::NewVideos);
                s_lock.new_videos.is_some()
            };
            if !has_cache {
                spawn_fetch_new_videos(state.clone(), ctx.clone());
            }
            true
        }
        PendingAction::GoToSettings => {
            let mut s_lock = state.lock().unwrap();
            s_lock.navigate_clear_history(View::Settings);
            true
        }
        PendingAction::GoToAbout => {
            let mut s_lock = state.lock().unwrap();
            s_lock.navigate_clear_history(View::About);
            true
        }
        _ => false,
    }
}
