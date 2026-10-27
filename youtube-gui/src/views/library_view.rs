use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::{scan_downloaded_items, DownloadedItem};

use crate::types::{lock_state, AppState, PendingAction};

#[derive(Default, Clone, PartialEq, Eq)]
pub enum LibraryFilter {
    #[default]
    All,
    Audio,
    Video,
}

#[derive(Default)]
pub struct LibraryViewState {
    pub filter: LibraryFilter,
    pub search_query: String,
    pub delete_confirm: Option<PathBuf>,
}

pub fn render_library_view(
    state: &Arc<Mutex<AppState>>,
    ui: &mut egui::Ui,
    view_state: &mut LibraryViewState,
) -> Option<PendingAction> {
    let mut action = None;
    let (downloads_dir, playback_positions) = {
        let s = lock_state(state);
        (s.downloads_dir.clone(), s.playback_positions.clone())
    };

    let all_items = scan_downloaded_items(&downloads_dir);
    let total_bytes: u64 = all_items.iter().map(|i| i.size_bytes).sum();
    let total_mb = (total_bytes as f64) / 1024.0 / 1024.0;

    ui.horizontal(|ui| {
        ui.heading(
            egui::RichText::new("📁 Offline Media Library")
                .size(22.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.add_space(20.0);
        if ui
            .button("📂 Open Folder")
            .on_hover_text("Open downloads folder in system file manager")
            .clicked()
        {
            action = Some(PendingAction::OpenFolder {
                path: downloads_dir.clone(),
            });
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Total: {} files ({total_mb:.1} MB) in {}",
                all_items.len(),
                downloads_dir.display()
            ))
            .color(egui::Color32::GRAY)
            .small(),
        );
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // Filter controls row
    ui.horizontal(|ui| {
        ui.label("Filter:");
        if ui
            .selectable_label(view_state.filter == LibraryFilter::All, "All")
            .clicked()
        {
            view_state.filter = LibraryFilter::All;
        }
        if ui
            .selectable_label(view_state.filter == LibraryFilter::Audio, "🎵 Audio (MP3)")
            .clicked()
        {
            view_state.filter = LibraryFilter::Audio;
        }
        if ui
            .selectable_label(view_state.filter == LibraryFilter::Video, "🎬 Video (MP4)")
            .clicked()
        {
            view_state.filter = LibraryFilter::Video;
        }

        ui.add_space(20.0);
        ui.label("🔍 Search:");
        ui.text_edit_singleline(&mut view_state.search_query);
        if !view_state.search_query.is_empty() && ui.button("✖").clicked() {
            view_state.search_query.clear();
        }
    });

    ui.add_space(10.0);

    // Filter items based on active criteria
    let query_lower = view_state.search_query.trim().to_lowercase();
    let filtered: Vec<&DownloadedItem> = all_items
        .iter()
        .filter(|item| match view_state.filter {
            LibraryFilter::All => true,
            LibraryFilter::Audio => item.is_audio,
            LibraryFilter::Video => !item.is_audio,
        })
        .filter(|item| {
            if query_lower.is_empty() {
                true
            } else {
                item.file_name.to_lowercase().contains(&query_lower)
                    || item
                        .video_id
                        .as_deref()
                        .is_some_and(|id| id.to_lowercase().contains(&query_lower))
            }
        })
        .collect();

    if filtered.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            if all_items.is_empty() {
                ui.label(
                    egui::RichText::new("No downloaded media found.")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
                ui.label("Download videos or audio from YouTube feeds to view them here offline.");
            } else {
                ui.label(
                    egui::RichText::new("No downloaded files match your search filter.")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
            }
        });
        return action;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in filtered {
            ui.push_id(&item.path, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Type Icon
                        let icon = if item.is_audio { "🎵" } else { "🎬" };
                        ui.label(egui::RichText::new(icon).size(24.0));
                        ui.add_space(8.0);

                        // File info column
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&item.file_name)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );

                            let size_mb = (item.size_bytes as f64) / 1024.0 / 1024.0;
                            let progress_info = item
                                .video_id
                                .as_ref()
                                .and_then(|id| playback_positions.get(id));

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{size_mb:.1} MB"))
                                        .color(egui::Color32::LIGHT_BLUE)
                                        .small(),
                                );

                                if let Some(vid) = &item.video_id {
                                    ui.label(
                                        egui::RichText::new(format!("ID: {vid}"))
                                            .color(egui::Color32::DARK_GRAY)
                                            .small(),
                                    );
                                }

                                if let Some(prog) = progress_info {
                                    let format_time = |secs: f32| -> String {
                                        let total = secs.max(0.0) as u32;
                                        format!("{:02}:{:02}", total / 60, total % 60)
                                    };
                                    let pos_str = format_time(prog.position_secs);
                                    let dur_str = if prog.duration_secs > 0.0 {
                                        format_time(prog.duration_secs)
                                    } else {
                                        "--:--".to_string()
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("⏱ Watched: {pos_str} / {dur_str}"))
                                            .color(egui::Color32::from_rgb(255, 180, 80))
                                            .small(),
                                    );
                                }
                            });
                        });

                        // Right-aligned actions
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Delete button with confirmation
                            if view_state.delete_confirm.as_ref() == Some(&item.path) {
                                if ui
                                    .button(egui::RichText::new("Confirm 🗑").color(egui::Color32::RED))
                                    .clicked()
                                {
                                    action = Some(PendingAction::DeleteDownloadedFile {
                                        path: item.path.clone(),
                                        video_id: item.video_id.clone(),
                                    });
                                    view_state.delete_confirm = None;
                                }
                                if ui.button("Cancel").clicked() {
                                    view_state.delete_confirm = None;
                                }
                            } else if ui
                                .button("🗑")
                                .on_hover_text("Delete downloaded file from disk")
                                .clicked()
                            {
                                view_state.delete_confirm = Some(item.path.clone());
                            }

                            // Reveal in folder
                            if ui
                                .button("📂")
                                .on_hover_text("Show file in folder")
                                .clicked()
                            {
                                action = Some(PendingAction::OpenFolder {
                                    path: item.path.clone(),
                                });
                            }

                            // Play / Resume
                            let start_secs = item
                                .video_id
                                .as_ref()
                                .and_then(|id| playback_positions.get(id))
                                .map(|p| p.position_secs);

                            let play_label = if let Some(secs) = start_secs {
                                if secs > 2.0 {
                                    let total = secs as u32;
                                    format!("▶ Resume ({:02}:{:02})", total / 60, total % 60)
                                } else {
                                    "▶ Play".to_string()
                                }
                            } else {
                                "▶ Play".to_string()
                            };

                            if ui
                                .button(
                                    egui::RichText::new(play_label)
                                        .color(egui::Color32::from_rgb(100, 220, 100)),
                                )
                                .clicked()
                            {
                                action = Some(PendingAction::PlayLocal {
                                    path: item.path.clone(),
                                    title: item.file_name.clone(),
                                    video_id: item.video_id.clone(),
                                    start_secs,
                                });
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
