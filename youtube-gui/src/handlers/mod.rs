//! # GUI Domain Action Handlers (Single Responsibility Principle)
//!
//! Decomposes the monolithic UI action match in `main.rs` into focused,
//! domain-specific handlers for authentication, navigation, playback, downloads,
//! library operations, and application settings.

pub mod auth;
pub mod download;
pub mod library;
pub mod navigation;
pub mod playback;
pub mod settings;

use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};

use crate::types::{AppState, PendingAction, PlayerCommand};

/// Dispatches a pending UI action to the appropriate domain handler.
pub fn dispatch_pending_action(
    action: PendingAction,
    state: &Arc<Mutex<AppState>>,
    audio_tx: &Sender<PlayerCommand>,
    ctx: &egui::Context,
) {
    if action == PendingAction::None {
        return;
    }

    // 1. Navigation actions
    if navigation::handle_navigation_action(&action, state, ctx) {
        return;
    }

    // 2. Auth & session actions
    if auth::handle_auth_action(&action, state, ctx) {
        return;
    }

    // 3. Playback & streaming actions
    if playback::handle_playback_action(&action, state, audio_tx) {
        return;
    }

    // 4. Download actions
    if download::handle_download_action(&action, state, ctx) {
        return;
    }

    // 5. Settings actions
    if settings::handle_settings_action(&action, state) {
        return;
    }

    // 6. Media library, feed, playlist, comment, and subscription actions
    let _ = library::handle_library_action(&action, state, ctx);
}
