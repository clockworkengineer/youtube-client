use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::DownloadStatus;

use crate::types::{AppState, PendingAction, lock_state};

#[derive(Default)]
pub struct HistoryViewState {
    pub search_query: String,
    pub clear_confirm: bool,
}

pub fn render_history_view(
    state: &Arc<Mutex<AppState>>,
    ui: &mut egui::Ui,
    view_state: &mut HistoryViewState,
) -> Option<PendingAction> {
    let mut action = None;
    let (playback_positions, downloads) = {
        let s = lock_state(state);
        (s.playback_positions.clone(), s.downloads.clone())
    };

    let mut entries: Vec<(String, youtube_client_lib::utils::PlaybackProgress)> =
        playback_positions.into_iter().collect();
    entries.sort_by_key(|a| std::cmp::Reverse(a.1.updated_at));

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("⏱ Watch Progress History")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!("({} videos)", entries.len()))
                .size(13.0)
                .color(egui::Color32::GRAY),
        );
        ui.add_space(16.0);

        if !entries.is_empty() {
            if view_state.clear_confirm {
                ui.label(
                    egui::RichText::new("Clear all history?")
                        .color(egui::Color32::from_rgb(255, 180, 100)),
                );
                if ui
                    .button(
                        egui::RichText::new("✔ Confirm Clear")
                            .color(egui::Color32::from_rgb(255, 100, 100))
                            .strong(),
                    )
                    .clicked()
                {
                    action = Some(PendingAction::ClearHistory);
                    view_state.clear_confirm = false;
                }
                if ui.button("Cancel").clicked() {
                    view_state.clear_confirm = false;
                }
            } else if ui
                .button("🗑 Clear All")
                .on_hover_text("Purge all saved watch progress timestamps")
                .clicked()
            {
                view_state.clear_confirm = true;
            }
        }
    });

    ui.add_space(8.0);

    // Search bar
    ui.horizontal(|ui| {
        ui.label("🔎 Filter History:");
        ui.add(
            egui::TextEdit::singleline(&mut view_state.search_query)
                .hint_text("Search by title or Video ID...")
                .desired_width(240.0),
        );
        if !view_state.search_query.is_empty()
            && ui.button("✖ Clear").on_hover_text("Clear filter").clicked()
        {
            view_state.search_query.clear();
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    let query_lower = view_state.search_query.trim().to_lowercase();
    let filtered: Vec<&(String, youtube_client_lib::utils::PlaybackProgress)> = entries
        .iter()
        .filter(|(vid, _)| {
            if query_lower.is_empty() {
                true
            } else {
                let local_title = match downloads.get(vid) {
                    Some(DownloadStatus::Finished(path)) => path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_lowercase())
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                vid.to_lowercase().contains(&query_lower) || local_title.contains(&query_lower)
            }
        })
        .collect();

    if filtered.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            if entries.is_empty() {
                ui.label(
                    egui::RichText::new("No watch history recorded yet.")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
                ui.label(
                    "Videos you stream or play locally will automatically save progress here.",
                );
            } else {
                ui.label(
                    egui::RichText::new("No entries match your filter.")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
            }
        });
        return action;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (video_id, progress) in filtered {
            ui.push_id(video_id, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Check if we have a downloaded local file for this video
                        let local_path = match downloads.get(video_id) {
                            Some(DownloadStatus::Finished(path)) if path.exists() => {
                                Some(path.clone())
                            }
                            _ => None,
                        };

                        // Left column: Video details and progress bar
                        ui.vertical(|ui| {
                            let title = if let Some(ref path) = local_path {
                                path.file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| video_id.clone())
                            } else {
                                format!("YouTube Video ({video_id})")
                            };

                            ui.label(
                                egui::RichText::new(&title)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .on_hover_text(format!("Video ID: {video_id}"));

                            let format_time = |secs: f32| -> String {
                                let total = secs.max(0.0) as u32;
                                format!("{:02}:{:02}", total / 60, total % 60)
                            };

                            let pos_str = format_time(progress.position_secs);
                            let dur_str = if progress.duration_secs > 0.0 {
                                format_time(progress.duration_secs)
                            } else {
                                "--:--".to_string()
                            };

                            let ratio = if progress.duration_secs > 0.0 {
                                (progress.position_secs / progress.duration_secs).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };

                            ui.add_space(2.0);
                            let bar_text = format!("{pos_str} / {dur_str} ({:.0}%)", ratio * 100.0);
                            ui.add(
                                egui::ProgressBar::new(ratio)
                                    .text(bar_text)
                                    .desired_width(240.0)
                                    .animate(false),
                            );

                            ui.horizontal(|ui| {
                                if local_path.is_some() {
                                    ui.label(
                                        egui::RichText::new("💾 Offline File")
                                            .color(egui::Color32::LIGHT_GREEN)
                                            .small(),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new("🌐 Online Stream")
                                            .color(egui::Color32::LIGHT_BLUE)
                                            .small(),
                                    );
                                }

                                if progress.updated_at > 0 {
                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(format_relative_time(
                                            now_secs,
                                            progress.updated_at,
                                        ))
                                        .color(egui::Color32::GRAY)
                                        .small(),
                                    );
                                }
                            });
                        });

                        // Right column: Actions
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Remove single item
                            if ui
                                .button("🗑 Remove")
                                .on_hover_text("Remove entry from watch history")
                                .clicked()
                            {
                                action = Some(PendingAction::RemoveHistoryItem {
                                    video_id: video_id.clone(),
                                });
                            }

                            // Restart from 0:00
                            if ui
                                .button("↺ Restart")
                                .on_hover_text("Play from beginning (0:00)")
                                .clicked()
                            {
                                if let Some(ref path) = local_path {
                                    action = Some(PendingAction::PlayLocal {
                                        path: path.clone(),
                                        title: video_id.clone(),
                                        video_id: Some(video_id.clone()),
                                        start_secs: Some(0.0),
                                    });
                                } else {
                                    action = Some(PendingAction::StreamVideo {
                                        video_id: video_id.clone(),
                                        start_secs: Some(0.0),
                                    });
                                }
                            }

                            // Resume
                            let resume_secs = progress.position_secs;
                            if ui
                                .button(
                                    egui::RichText::new("▶ Resume")
                                        .color(egui::Color32::from_rgb(100, 220, 100))
                                        .strong(),
                                )
                                .on_hover_text("Resume playback from last position")
                                .clicked()
                            {
                                if let Some(ref path) = local_path {
                                    action = Some(PendingAction::PlayLocal {
                                        path: path.clone(),
                                        title: video_id.clone(),
                                        video_id: Some(video_id.clone()),
                                        start_secs: Some(resume_secs),
                                    });
                                } else {
                                    action = Some(PendingAction::StreamVideo {
                                        video_id: video_id.clone(),
                                        start_secs: Some(resume_secs),
                                    });
                                }
                            }
                        });
                    });
                });
            });
            ui.add_space(4.0);
        }
    });

    action
}

fn format_relative_time(now_secs: u64, updated_at: u64) -> String {
    let diff = now_secs.saturating_sub(updated_at);
    if diff < 60 {
        "Watched just now".to_string()
    } else if diff < 3_600 {
        format!("Watched {}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("Watched {}h ago", diff / 3_600)
    } else {
        format!("Watched {}d ago", diff / 86_400)
    }
}
