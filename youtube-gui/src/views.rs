//! # egui UI View & Video Card Renderers
//!
//! Provides egui view layout components, thumbnail texture caching, video cards,
//! error recovery banners, and contextual help badges.

pub mod about_view;
pub mod channel_view;
pub mod details_view;
pub mod history_view;
pub mod library_view;
pub mod login_view;
pub mod new_videos_view;
pub mod playlist_videos_view;
pub mod playlists_view;
pub mod search_view;
pub mod settings_view;
pub mod subscriptions_view;
pub mod traits;

pub use about_view::*;
pub use channel_view::*;
pub use details_view::*;
pub use history_view::*;
pub use library_view::*;
pub use login_view::*;
pub use new_videos_view::*;
pub use playlist_videos_view::*;
pub use playlists_view::*;
pub use search_view::*;
pub use settings_view::*;
pub use subscriptions_view::*;
#[allow(unused_imports)]
pub use traits::*;

use eframe::egui;
use youtube_client_lib::Video;
use youtube_client_lib::utils::DownloadStatus;

use crate::actions::fetch_thumbnail;
use crate::types::{AppState, PendingAction, PlayerCommand, Thumbnail};

/// Render a contextual `(?)` help badge that displays a plain-language tooltip on hover (Heuristic 10).
pub fn help_badge(ui: &mut egui::Ui, tooltip: &str) {
    ui.label(
        egui::RichText::new("ⓘ")
            .size(13.0)
            .color(egui::Color32::from_rgb(130, 175, 240)),
    )
    .on_hover_text(tooltip);
}

/// Diagnose a raw error string into a plain-language summary and actionable recovery advice (Heuristic 9).
pub fn diagnose_error(raw_error: &str) -> (&'static str, &'static str, bool) {
    let lower = raw_error.to_lowercase();
    if lower.contains("quota") || lower.contains("403") {
        (
            "Daily YouTube API quota limit reached or access denied.",
            "Wait until midnight Pacific Time for your quota to reset, or configure your own Google Cloud OAuth Client ID in Settings.",
            false,
        )
    } else if lower.contains("not signed in")
        || lower.contains("authorization expired")
        || lower.contains("authentication failed")
        || lower.contains("token")
        || lower.contains("401")
        || lower.contains("oauth")
    {
        (
            "Your Google sign-in session has expired or is missing permissions.",
            "Click 'Sign In Again' below to re-authorize your Google account in the browser.",
            true,
        )
    } else if lower.contains("yt-dlp") || lower.contains("ffmpeg") {
        (
            "Required media helper tool (yt-dlp or ffmpeg) was not found.",
            "Install yt-dlp and ffmpeg (e.g. `winget install yt-dlp.yt-dlp` on Windows or `brew install yt-dlp` on macOS) and restart the app.",
            false,
        )
    } else if lower.contains("dns")
        || lower.contains("timeout")
        || lower.contains("connect")
        || lower.contains("network")
    {
        (
            "Could not reach YouTube servers due to a network connection issue.",
            "Check your internet connection or firewall settings, then click Retry.",
            false,
        )
    } else {
        (
            "Something went wrong while communicating with YouTube.",
            "Try clicking Retry below, or check Settings if the issue persists.",
            false,
        )
    }
}

/// Render a structured, plain-language error banner with recovery actions and collapsible technical details (Heuristic 9).
pub fn render_error_banner(
    ui: &mut egui::Ui,
    heading: &str,
    raw_error: &str,
    retry_action: PendingAction,
    action_out: &mut PendingAction,
) {
    let (summary, hint, needs_auth) = diagnose_error(raw_error);
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(45, 25, 28))
        .stroke(egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgb(180, 70, 70),
        ))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠")
                        .size(20.0)
                        .color(egui::Color32::from_rgb(255, 110, 110)),
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(heading)
                            .size(15.0)
                            .strong()
                            .color(egui::Color32::from_rgb(255, 160, 160)),
                    );
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(summary).color(egui::Color32::WHITE));
                    ui.label(
                        egui::RichText::new(format!("💡 Suggestion: {hint}"))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(210, 210, 220)),
                    );
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if !matches!(retry_action, PendingAction::None)
                    && ui
                        .button(egui::RichText::new("🔄 Retry").strong())
                        .on_hover_text("Try this operation again")
                        .clicked()
                {
                    *action_out = retry_action;
                }
                if needs_auth
                    && ui
                        .button(
                            egui::RichText::new("🔐 Sign In Again")
                                .strong()
                                .color(egui::Color32::from_rgb(130, 200, 255)),
                        )
                        .on_hover_text("Re-authenticate with your Google account")
                        .clicked()
                {
                    *action_out = PendingAction::SpawnDefaultLogin;
                }
                if ui
                    .button("⚙ Open Settings")
                    .on_hover_text("Check configuration, credentials, or quota")
                    .clicked()
                {
                    *action_out = PendingAction::GoToSettings;
                }
            });

            ui.add_space(4.0);
            ui.collapsing(
                egui::RichText::new("Technical Details")
                    .size(11.0)
                    .color(egui::Color32::GRAY),
                |ui| {
                    ui.label(
                        egui::RichText::new(raw_error)
                            .monospace()
                            .size(11.0)
                            .color(egui::Color32::LIGHT_GRAY),
                    );
                },
            );
        });
}

/// Render a single video card inside a playlist with an optional "Remove from Playlist" action.
#[allow(clippy::too_many_arguments)]
pub fn draw_video_card_in_playlist(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &std::sync::mpsc::Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    action: &mut PendingAction,
    can_dismiss: bool,
    playlist_context: Option<(&str, &str)>,
) {
    ui.push_id(&video.id, |ui| {
        let texture =
            get_or_fetch_thumbnail(state, http_client, ctx, &video.id, &video.thumbnail_url);
        let (download_status, is_playing, is_buffering_this, playback_progress) = {
            let s_lock = state.lock().unwrap();
            let is_this_video =
                s_lock.player_state.current_video_id.as_deref() == Some(video.id.as_str())
                    || s_lock.player_state.current_title == video.title;
            let is_playing = s_lock.player_state.playing && is_this_video;
            let is_buffering_this = s_lock.player_state.buffering && is_this_video;
            (
                s_lock
                    .downloads
                    .get(&video.id)
                    .cloned()
                    .unwrap_or(DownloadStatus::NotStarted),
                is_playing,
                is_buffering_this,
                s_lock.playback_positions.get(&video.id).cloned(),
            )
        };
        let mut card_clicked = false;

        let _response = ui.group(|ui| {
            ui.horizontal(|ui| {
                let left_response = ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        if let Some(tex) = &texture {
                            ui.add(
                                egui::Image::from_texture(tex)
                                    .max_width(100.0)
                                    .max_height(100.0),
                            );
                        } else {
                            let (rect, _response) = ui
                                .allocate_exact_size(egui::vec2(100.0, 75.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                rect,
                                4.0,
                                egui::Color32::from_rgb(50, 53, 60),
                            );
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "🎬",
                                egui::FontId::proportional(40.0),
                                egui::Color32::LIGHT_GRAY,
                            );
                        }
                        if let Some(ref prog) = playback_progress {
                            if prog.duration_secs > 0.0 && prog.position_secs > 2.0 {
                                let pct = (prog.position_secs / prog.duration_secs).clamp(0.0, 1.0);
                                ui.add(
                                    egui::ProgressBar::new(pct)
                                        .desired_width(100.0)
                                        .desired_height(4.0)
                                        .fill(egui::Color32::from_rgb(255, 60, 60)),
                                );
                            }
                        }
                    });

                    ui.add_space(15.0);

                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&video.title)
                                .size(15.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if !video.channel_title.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!("👤 {}", video.channel_title))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(175, 190, 215)),
                                );
                                ui.add_space(12.0);
                            }
                            let date = if video.published_at.len() >= 10 {
                                &video.published_at[..10]
                            } else {
                                &video.published_at
                            };
                            if !date.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!("📅 {date}"))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(140, 140, 150)),
                                );
                            }
                        });
                        if let Some(ref prog) = playback_progress {
                            if prog.position_secs > 5.0 {
                                let m = (prog.position_secs as u32) / 60;
                                let s = (prog.position_secs as u32) % 60;
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(format!("⏱ Watched to {m:02}:{s:02}"))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(255, 120, 120)),
                                );
                            }
                        }
                    });
                });

                let left_interact = ui
                    .interact(
                        left_response.response.rect,
                        left_response.response.id.with("click"),
                        egui::Sense::click(),
                    )
                    .on_hover_text(format!(
                        "Click to view details, description & comments (Video ID: {})",
                        video.id
                    ));
                if left_interact.clicked() {
                    card_clicked = true;
                }
                if left_interact.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some((pl_id, pl_title)) = playlist_context {
                        if let Some(ref item_id) = video.playlist_item_id {
                            if ui
                                .button(
                                    egui::RichText::new("🗑 Remove")
                                        .color(egui::Color32::from_rgb(255, 130, 130)),
                                )
                                .on_hover_text(format!("Remove this video from '{pl_title}'"))
                                .clicked()
                            {
                                *action = PendingAction::RemoveFromPlaylist {
                                    playlist_id: pl_id.to_string(),
                                    playlist_title: pl_title.to_string(),
                                    playlist_item_id: item_id.clone(),
                                    video_id: video.id.clone(),
                                };
                            }
                        }
                    }

                    if can_dismiss
                        && ui
                            .button("✖ Dismiss")
                            .on_hover_text("Hide from New Videos feed (can be undone)")
                            .clicked()
                    {
                        *action = PendingAction::DismissNewVideo {
                            video_id: video.id.clone(),
                        };
                    }
                    match &download_status {
                        DownloadStatus::NotStarted => {
                            if ui
                                .button("📥 Download")
                                .on_hover_text("Download MP4 video to your local Library")
                                .clicked()
                            {
                                *action = PendingAction::SpawnDownload {
                                    video: video.clone(),
                                    is_audio: false,
                                    resolution: None,
                                    audio_bitrate: None,
                                };
                            }
                            if is_buffering_this {
                                if ui
                                    .button(
                                        egui::RichText::new("⏹ Cancel Audio")
                                            .color(egui::Color32::from_rgb(255, 160, 100)),
                                    )
                                    .on_hover_text("Cancel audio stream buffering")
                                    .clicked()
                                {
                                    let _ = audio_tx.send(PlayerCommand::Stop);
                                }
                                ui.spinner();
                            } else if is_playing {
                                if ui
                                    .button(
                                        egui::RichText::new("⏹ Stop Audio")
                                            .color(egui::Color32::from_rgb(255, 100, 100))
                                            .strong(),
                                    )
                                    .on_hover_text("Stop audio playback")
                                    .clicked()
                                {
                                    let _ = audio_tx.send(PlayerCommand::Stop);
                                }
                            } else if ui
                                .button("🎧 Listen")
                                .on_hover_text("Stream audio directly in-app")
                                .clicked()
                            {
                                *action = PendingAction::StreamAudioDirect {
                                    video_id: video.id.clone(),
                                    title: video.title.clone(),
                                    start_secs: playback_progress.as_ref().map(|p| p.position_secs),
                                };
                            }
                        }
                        DownloadStatus::Downloading { progress } => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                if let Some(fraction) = parse_progress_percentage(progress) {
                                    ui.add(
                                        egui::ProgressBar::new(fraction)
                                            .show_percentage()
                                            .desired_width(100.0),
                                    );
                                } else {
                                    ui.label(progress);
                                }
                            });
                        }
                        DownloadStatus::Finished(_) => {
                            let path_opt = match &download_status {
                                DownloadStatus::Finished(path) => Some(path),
                                _ => None,
                            };
                            let is_mp3 = path_opt
                                .map(|p| p.extension().map(|ext| ext == "mp3").unwrap_or(false))
                                .unwrap_or(false);

                            if is_mp3 {
                                if is_playing {
                                    if ui
                                        .button(
                                            egui::RichText::new("⏹ Stop")
                                                .color(egui::Color32::from_rgb(255, 100, 100))
                                                .strong(),
                                        )
                                        .clicked()
                                    {
                                        let _ = audio_tx.send(PlayerCommand::Stop);
                                    }
                                } else {
                                    let resume_sec =
                                        playback_progress.as_ref().map(|p| p.position_secs);
                                    if let Some(sec) = resume_sec.filter(|&s| s > 5.0) {
                                        let m = (sec as u32) / 60;
                                        let s = (sec as u32) % 60;
                                        if ui.button(format!("▶ Resume ({m:02}:{s:02})")).clicked()
                                        {
                                            if let Some(path) = path_opt {
                                                *action = PendingAction::PlayLocal {
                                                    path: path.clone(),
                                                    title: video.title.clone(),
                                                    video_id: Some(video.id.clone()),
                                                    start_secs: Some(sec),
                                                };
                                            }
                                        }
                                        if ui.button("↺ Start").clicked() {
                                            if let Some(path) = path_opt {
                                                *action = PendingAction::PlayLocal {
                                                    path: path.clone(),
                                                    title: video.title.clone(),
                                                    video_id: Some(video.id.clone()),
                                                    start_secs: None,
                                                };
                                            }
                                        }
                                    } else if ui.button("▶ Play Local").clicked() {
                                        if let Some(path) = path_opt {
                                            *action = PendingAction::PlayLocal {
                                                path: path.clone(),
                                                title: video.title.clone(),
                                                video_id: Some(video.id.clone()),
                                                start_secs: None,
                                            };
                                        }
                                    }
                                }
                            } else {
                                let resume_sec =
                                    playback_progress.as_ref().map(|p| p.position_secs);
                                if let Some(sec) = resume_sec.filter(|&s| s > 5.0) {
                                    let m = (sec as u32) / 60;
                                    let s = (sec as u32) % 60;
                                    if ui
                                        .button(format!("▶ Resume Video ({m:02}:{s:02})"))
                                        .clicked()
                                    {
                                        if let Some(path) = path_opt {
                                            *action = PendingAction::PlayLocal {
                                                path: path.clone(),
                                                title: video.title.clone(),
                                                video_id: Some(video.id.clone()),
                                                start_secs: Some(sec),
                                            };
                                        }
                                    }
                                    if ui.button("↺ Start").clicked() {
                                        if let Some(path) = path_opt {
                                            *action = PendingAction::PlayLocal {
                                                path: path.clone(),
                                                title: video.title.clone(),
                                                video_id: Some(video.id.clone()),
                                                start_secs: None,
                                            };
                                        }
                                    }
                                } else if ui.button("▶ Play Local Video").clicked() {
                                    if let Some(path) = path_opt {
                                        *action = PendingAction::PlayLocal {
                                            path: path.clone(),
                                            title: video.title.clone(),
                                            video_id: Some(video.id.clone()),
                                            start_secs: None,
                                        };
                                    }
                                }
                            }
                        }
                        DownloadStatus::Failed(err) => {
                            let is_missing_ytdlp = err.contains("yt-dlp");
                            if ui
                                .button("🔄 Retry")
                                .on_hover_text("Retry downloading this video")
                                .clicked()
                            {
                                *action = PendingAction::SpawnDownload {
                                    video: video.clone(),
                                    is_audio: false,
                                    resolution: None,
                                    audio_bitrate: None,
                                };
                            }
                            let fail_text = if is_missing_ytdlp {
                                "⚠ yt-dlp missing"
                            } else {
                                "⚠ Download failed"
                            };
                            ui.label(
                                egui::RichText::new(fail_text).color(egui::Color32::LIGHT_RED),
                            )
                            .on_hover_text(format!(
                                "{err}\n\nTip: Ensure yt-dlp and ffmpeg are installed and up to date."
                            ));
                        }
                    }
                });
            });
        });

        if card_clicked && matches!(action, PendingAction::None) {
            *action = PendingAction::LoadVideoDetails {
                video: video.clone(),
            };
        }
    });
}

/// Render a single video card with optional dismiss button ("Dismiss").
#[allow(clippy::too_many_arguments)]
pub fn draw_video_card_with_dismiss(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &std::sync::mpsc::Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    action: &mut PendingAction,
    can_dismiss: bool,
) {
    draw_video_card_in_playlist(
        state,
        http_client,
        audio_tx,
        ui,
        ctx,
        video,
        action,
        can_dismiss,
        None,
    );
}

pub fn draw_video_card(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &std::sync::mpsc::Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    video: &Video,
    action: &mut PendingAction,
) {
    draw_video_card_with_dismiss(state, http_client, audio_tx, ui, ctx, video, action, false);
}

pub fn get_or_fetch_thumbnail(
    state: &std::sync::Arc<std::sync::Mutex<AppState>>,
    http_client: &reqwest::Client,
    ctx: &egui::Context,
    id: &str,
    url: &str,
) -> Option<egui::TextureHandle> {
    let mut start_fetch = false;
    let texture = {
        let mut s = state.lock().unwrap();
        if !s.thumbnails.contains_key(id) {
            s.insert_thumbnail(
                id.to_string(),
                Thumbnail {
                    texture: None,
                    loading: false,
                },
            );
        }
        let thumbnail_entry = s.thumbnails.get_mut(id).unwrap();

        if thumbnail_entry.texture.is_none() && !thumbnail_entry.loading && !url.is_empty() {
            thumbnail_entry.loading = true;
            start_fetch = true;
        }
        thumbnail_entry.texture.clone()
    };

    if start_fetch {
        fetch_thumbnail(
            ctx.clone(),
            state.clone(),
            http_client.clone(),
            id.to_string(),
            url.to_string(),
        );
    }
    texture
}

/// Parse download progress percentage string (e.g. `"[download]  45.2% of 12.34MiB at 2.50MiB/s"`).
pub fn parse_progress_percentage(progress: &str) -> Option<f32> {
    if let Some(percent_idx) = progress.find('%') {
        let before = &progress[..percent_idx];
        let num_str = before.split_whitespace().last()?;
        if let Ok(val) = num_str.parse::<f32>() {
            return Some((val / 100.0).clamp(0.0, 1.0));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_error_categories() {
        let (summary, _, needs_auth) = diagnose_error("Error 403: quotaExceeded");
        assert!(summary.contains("quota"));
        assert!(!needs_auth);

        let (summary, _, needs_auth) =
            diagnose_error("Authorization expired or missing permissions");
        assert!(summary.contains("sign-in"));
        assert!(needs_auth);

        let (summary, _, _) = diagnose_error("yt-dlp executable not found");
        assert!(summary.contains("yt-dlp"));
    }

    #[test]
    fn test_parse_progress_percentage() {
        assert_eq!(
            parse_progress_percentage("[download]  45.0% of 12.34MiB at 2.50MiB/s"),
            Some(0.45)
        );
        assert_eq!(parse_progress_percentage("Starting..."), None);
    }
}
