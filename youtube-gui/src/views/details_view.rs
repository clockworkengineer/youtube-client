use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::utils::DownloadStatus;
use youtube_client_lib::{Comment, Playlist, Video, VideoDetails};

use crate::types::{AppState, PendingAction, PlayerCommand, PlayerState};
use crate::views::get_or_fetch_thumbnail;

/// Encapsulated view context for rendering video details, eliminating parameter sprawl (ISP & Clean Architecture).
pub struct VideoDetailsContext<'a> {
    pub video: &'a Video,
    pub details: &'a Option<Result<Arc<VideoDetails>, String>>,
    pub comments: &'a Option<Result<Arc<Vec<Comment>>, String>>,
    pub comments_next_page_token: &'a Option<String>,
    pub loading_more_comments: bool,
    pub player_state: &'a PlayerState,
    pub playlists: &'a Option<Result<Arc<Vec<Playlist>>, String>>,
    pub playlist_action_status: &'a Option<Result<String, String>>,
    pub comment_input: &'a mut String,
}

pub fn render_details_view(
    state: &Arc<Mutex<AppState>>,
    http_client: &reqwest::Client,
    audio_tx: &Sender<PlayerCommand>,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    vctx: VideoDetailsContext<'_>,
) -> Option<PendingAction> {
    let VideoDetailsContext {
        video,
        details,
        comments,
        comments_next_page_token,
        loading_more_comments,
        player_state,
        playlists,
        playlist_action_status,
        comment_input,
    } = vctx;

    let mut action = None;

    ui.horizontal(|ui| {
        if ui.button("⬅ Back").clicked() {
            action = Some(PendingAction::GoBack);
        }
        ui.heading("🎬 Video Details");
    });
    ui.add_space(15.0);

    let (download_status, playback_progress, sponsor_segments) = {
        let s_lock = state.lock().unwrap();
        (
            s_lock
                .downloads
                .get(&video.id)
                .cloned()
                .unwrap_or(DownloadStatus::NotStarted),
            s_lock.playback_positions.get(&video.id).cloned(),
            s_lock.sponsor_segments.get(&video.id).cloned().unwrap_or_default(),
        )
    };

    ui.horizontal(|ui| {
        let texture =
            get_or_fetch_thumbnail(state, http_client, ctx, &video.id, &video.thumbnail_url);
        ui.vertical(|ui| {
            if let Some(tex) = &texture {
                let img = egui::Image::from_texture(tex)
                    .max_width(200.0)
                    .max_height(150.0)
                    .sense(egui::Sense::click());
                let img_response = ui.add(img);
                if img_response.clicked() {
                    let start = playback_progress.as_ref().map(|p| p.position_secs);
                    action = Some(PendingAction::StreamVideo {
                        video_id: video.id.clone(),
                        start_secs: start,
                    });
                }
                if img_response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    img_response.on_hover_text("Click to stream video");
                }
            } else {
                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(200.0, 150.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 53, 60));
            }

            if let Some(ref prog) = playback_progress {
                if prog.duration_secs > 0.0 && prog.position_secs > 2.0 {
                    let pct = (prog.position_secs / prog.duration_secs).clamp(0.0, 1.0);
                    ui.add(
                        egui::ProgressBar::new(pct)
                            .desired_width(200.0)
                            .desired_height(4.0)
                            .fill(egui::Color32::from_rgb(255, 60, 60)),
                    );
                    let m = (prog.position_secs as u32) / 60;
                    let s = (prog.position_secs as u32) % 60;
                    let dur_m = (prog.duration_secs as u32) / 60;
                    let dur_s = (prog.duration_secs as u32) % 60;
                    ui.label(
                        egui::RichText::new(format!(
                            "⏱ Watched: {m:02}:{s:02} / {dur_m:02}:{dur_s:02}"
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(255, 120, 120)),
                    );
                }
            }
        });

        ui.add_space(20.0);

        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(&video.title)
                    .size(18.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);

            // Channel link and published timestamp
            ui.horizontal(|ui| {
                if let Some(Ok(d)) = details {
                    if ui
                        .link(
                            egui::RichText::new(format!("👤 {}", d.channel_title))
                                .strong()
                                .color(egui::Color32::from_rgb(120, 180, 255)),
                        )
                        .clicked()
                    {
                        action = Some(PendingAction::LoadChannel {
                            id: d.channel_id.clone(),
                            title: d.channel_title.clone(),
                            description: String::new(),
                        });
                    }
                    ui.separator();
                } else if !video.channel_title.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("👤 {}", video.channel_title))
                            .color(egui::Color32::from_rgb(180, 180, 190)),
                    );
                    ui.separator();
                }

                ui.label(
                    egui::RichText::new(format!("Published: {}", video.published_at))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(160, 160, 170)),
                );
            });

            // Statistics badges if details are loaded
            if let Some(Ok(d)) = details {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("👁 {} views", d.view_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("👍 {} likes", d.like_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    if let Some(dislikes) = d.dislike_count {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("👎 {dislikes} dislikes"))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(200, 200, 210)),
                        )
                        .on_hover_text("Crowd-sourced from Return YouTube Dislike (RYD) API");
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("⏱ {}", d.duration_formatted))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("💬 {} comments", d.comment_count))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 200, 210)),
                    );
                });
            }

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Video ID: {}", video.id))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(130, 130, 140)),
            );

            if !sponsor_segments.is_empty() {
                ui.add_space(6.0);
                let total_dur = details
                    .as_ref()
                    .and_then(|res| res.as_ref().ok())
                    .map(|d| d.duration_seconds as f64)
                    .unwrap_or_else(|| {
                        sponsor_segments
                            .iter()
                            .map(|s| s.end_secs)
                            .fold(0.0_f64, f64::max)
                    });

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "🛡️ SponsorBlock: {} segment(s) detected",
                                sponsor_segments.len()
                            ))
                            .size(12.0)
                            .strong()
                            .color(egui::Color32::from_rgb(100, 220, 120)),
                        );
                    });

                    if total_dur > 0.0 {
                        ui.add_space(2.0);
                        let bar_width = 340.0_f32;
                        let bar_height = 8.0_f32;
                        let (bar_rect, bar_resp) = ui.allocate_exact_size(
                            egui::vec2(bar_width, bar_height),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            bar_rect,
                            3.0,
                            egui::Color32::from_rgb(55, 60, 70),
                        );

                        for seg in &sponsor_segments {
                            let start_ratio = (seg.start_secs / total_dur).clamp(0.0, 1.0) as f32;
                            let end_ratio = (seg.end_secs / total_dur).clamp(0.0, 1.0) as f32;
                            let seg_min_x = bar_rect.min.x + start_ratio * bar_width;
                            let seg_max_x = (bar_rect.min.x + end_ratio * bar_width)
                                .max(seg_min_x + 2.0)
                                .min(bar_rect.max.x);
                            let seg_rect = egui::Rect::from_min_max(
                                egui::pos2(seg_min_x, bar_rect.min.y),
                                egui::pos2(seg_max_x, bar_rect.max.y),
                            );
                            let color = match seg.category {
                                youtube_client_lib::SegmentCategory::Sponsor => {
                                    egui::Color32::from_rgb(0, 212, 0)
                                }
                                youtube_client_lib::SegmentCategory::Selfpromo => {
                                    egui::Color32::from_rgb(255, 255, 0)
                                }
                                youtube_client_lib::SegmentCategory::Interaction => {
                                    egui::Color32::from_rgb(204, 0, 255)
                                }
                                youtube_client_lib::SegmentCategory::Intro => {
                                    egui::Color32::from_rgb(0, 255, 255)
                                }
                                youtube_client_lib::SegmentCategory::Outro => {
                                    egui::Color32::from_rgb(2, 2, 237)
                                }
                                youtube_client_lib::SegmentCategory::Preview => {
                                    egui::Color32::from_rgb(0, 143, 214)
                                }
                                youtube_client_lib::SegmentCategory::MusicOfftopic => {
                                    egui::Color32::from_rgb(255, 153, 0)
                                }
                                youtube_client_lib::SegmentCategory::Other => {
                                    egui::Color32::from_rgb(180, 180, 180)
                                }
                            };
                            ui.painter().rect_filled(seg_rect, 2.0, color);
                        }

                        let hover_summary = sponsor_segments
                            .iter()
                            .map(|seg| {
                                let sm = (seg.start_secs as u32) / 60;
                                let ss = (seg.start_secs as u32) % 60;
                                let em = (seg.end_secs as u32) / 60;
                                let es = (seg.end_secs as u32) % 60;
                                format!(
                                    "• {}: {:02}:{:02} – {:02}:{:02}",
                                    seg.category.display_name(),
                                    sm,
                                    ss,
                                    em,
                                    es
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        bar_resp.on_hover_text(hover_summary);
                    }
                });
            }

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                match &download_status {
                    DownloadStatus::NotStarted => {
                        ui.menu_button("📥 Download Video ▾", |ui| {
                            let resolutions = [
                                youtube_client_lib::download::VideoResolution::Best,
                                youtube_client_lib::download::VideoResolution::Res2160p,
                                youtube_client_lib::download::VideoResolution::Res1440p,
                                youtube_client_lib::download::VideoResolution::Res1080p,
                                youtube_client_lib::download::VideoResolution::Res720p,
                                youtube_client_lib::download::VideoResolution::Res480p,
                                youtube_client_lib::download::VideoResolution::Res360p,
                            ];
                            for res in resolutions {
                                if ui.button(res.as_str()).clicked() {
                                    action = Some(PendingAction::SpawnDownload {
                                        video: video.clone(),
                                        is_audio: false,
                                        resolution: Some(res),
                                        audio_bitrate: None,
                                    });
                                    ui.close_menu();
                                }
                            }
                        });

                        ui.menu_button("📥 Download Audio ▾", |ui| {
                            let bitrates = [
                                youtube_client_lib::download::AudioBitrate::Best,
                                youtube_client_lib::download::AudioBitrate::Kbps320,
                                youtube_client_lib::download::AudioBitrate::Kbps256,
                                youtube_client_lib::download::AudioBitrate::Kbps192,
                                youtube_client_lib::download::AudioBitrate::Kbps128,
                            ];
                            for br in bitrates {
                                if ui.button(br.as_str()).clicked() {
                                    action = Some(PendingAction::SpawnDownload {
                                        video: video.clone(),
                                        is_audio: true,
                                        resolution: None,
                                        audio_bitrate: Some(br),
                                    });
                                    ui.close_menu();
                                }
                            }
                        });

                        if ui
                            .button("🎧 Listen (Stream Audio)")
                            .on_hover_text("Stream audio directly into app without downloading")
                            .clicked()
                        {
                            action = Some(PendingAction::StreamAudioDirect {
                                video_id: video.id.clone(),
                                title: video.title.clone(),
                                start_secs: playback_progress.as_ref().map(|p| p.position_secs),
                            });
                        }
                    }
                    DownloadStatus::Downloading { progress } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            if let Some(fraction) =
                                crate::views::parse_progress_percentage(progress)
                            {
                                ui.add(
                                    egui::ProgressBar::new(fraction)
                                        .show_percentage()
                                        .desired_width(120.0),
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
                            let is_playing =
                                player_state.playing && player_state.current_title == video.title;
                            if is_playing {
                                if ui
                                    .button(
                                        egui::RichText::new("⏹ Stop Audio")
                                            .color(egui::Color32::from_rgb(255, 100, 100))
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    let _ = audio_tx.send(PlayerCommand::Stop);
                                }
                            } else if let Some(prog) = &playback_progress {
                                if prog.position_secs > 5.0 {
                                    let mins = (prog.position_secs / 60.0).floor() as u64;
                                    let secs = (prog.position_secs % 60.0).floor() as u64;
                                    if ui
                                        .button(
                                            egui::RichText::new(format!(
                                                "▶ Resume Audio ({mins:02}:{secs:02})"
                                            ))
                                            .color(egui::Color32::from_rgb(100, 255, 100))
                                            .strong(),
                                        )
                                        .clicked()
                                    {
                                        if let Some(path) = path_opt {
                                            action = Some(PendingAction::PlayLocal {
                                                path: path.clone(),
                                                title: video.title.clone(),
                                                video_id: Some(video.id.clone()),
                                                start_secs: Some(prog.position_secs),
                                            });
                                        }
                                    }
                                    if ui
                                        .button(
                                            egui::RichText::new("↺ Start")
                                                .color(egui::Color32::from_rgb(180, 220, 180)),
                                        )
                                        .clicked()
                                    {
                                        if let Some(path) = path_opt {
                                            action = Some(PendingAction::PlayLocal {
                                                path: path.clone(),
                                                title: video.title.clone(),
                                                video_id: Some(video.id.clone()),
                                                start_secs: Some(0.0),
                                            });
                                        }
                                    }
                                } else if ui
                                    .button(
                                        egui::RichText::new("▶ Play Local Audio")
                                            .color(egui::Color32::from_rgb(100, 255, 100))
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = path_opt {
                                        action = Some(PendingAction::PlayLocal {
                                            path: path.clone(),
                                            title: video.title.clone(),
                                            video_id: Some(video.id.clone()),
                                            start_secs: None,
                                        });
                                    }
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new("▶ Play Local Audio")
                                        .color(egui::Color32::from_rgb(100, 255, 100))
                                        .strong(),
                                )
                                .clicked()
                            {
                                if let Some(path) = path_opt {
                                    action = Some(PendingAction::PlayLocal {
                                        path: path.clone(),
                                        title: video.title.clone(),
                                        video_id: Some(video.id.clone()),
                                        start_secs: None,
                                    });
                                }
                            }
                        } else if let Some(prog) = &playback_progress {
                            if prog.position_secs > 5.0 {
                                let mins = (prog.position_secs / 60.0).floor() as u64;
                                let secs = (prog.position_secs % 60.0).floor() as u64;
                                if ui
                                    .button(
                                        egui::RichText::new(format!(
                                            "▶ Resume Video ({mins:02}:{secs:02})"
                                        ))
                                        .color(egui::Color32::from_rgb(100, 255, 100))
                                        .strong(),
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = path_opt {
                                        action = Some(PendingAction::PlayLocal {
                                            path: path.clone(),
                                            title: video.title.clone(),
                                            video_id: Some(video.id.clone()),
                                            start_secs: Some(prog.position_secs),
                                        });
                                    }
                                }
                                if ui
                                    .button(
                                        egui::RichText::new("↺ Start")
                                            .color(egui::Color32::from_rgb(180, 220, 180)),
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = path_opt {
                                        action = Some(PendingAction::PlayLocal {
                                            path: path.clone(),
                                            title: video.title.clone(),
                                            video_id: Some(video.id.clone()),
                                            start_secs: Some(0.0),
                                        });
                                    }
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new("▶ Play Local Video")
                                        .color(egui::Color32::from_rgb(100, 255, 100))
                                        .strong(),
                                )
                                .clicked()
                            {
                                if let Some(path) = path_opt {
                                    action = Some(PendingAction::PlayLocal {
                                        path: path.clone(),
                                        title: video.title.clone(),
                                        video_id: Some(video.id.clone()),
                                        start_secs: None,
                                    });
                                }
                            }
                        } else if ui
                            .button(
                                egui::RichText::new("▶ Play Local Video")
                                    .color(egui::Color32::from_rgb(100, 255, 100))
                                    .strong(),
                            )
                            .clicked()
                        {
                            if let Some(path) = path_opt {
                                action = Some(PendingAction::PlayLocal {
                                    path: path.clone(),
                                    title: video.title.clone(),
                                    video_id: Some(video.id.clone()),
                                    start_secs: None,
                                });
                            }
                        }
                    }
                    DownloadStatus::Failed(err) => {
                        let is_missing_ytdlp = err.contains("yt-dlp");
                        if ui.button("❌ Retry Video").clicked() {
                            action = Some(PendingAction::SpawnDownload {
                                video: video.clone(),
                                is_audio: false,
                                resolution: None,
                                audio_bitrate: None,
                            });
                        }
                        if ui.button("❌ Retry Audio").clicked() {
                            action = Some(PendingAction::SpawnDownload {
                                video: video.clone(),
                                is_audio: true,
                                resolution: None,
                                audio_bitrate: None,
                            });
                        }
                        let fail_text = if is_missing_ytdlp {
                            "⚠️ yt-dlp missing (Install via winget / brew)"
                        } else {
                            "Failed"
                        };
                        ui.label(egui::RichText::new(fail_text).color(egui::Color32::LIGHT_RED))
                            .on_hover_text(err);
                    }
                }

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if let Some(prog) = &playback_progress {
                        if prog.position_secs > 5.0 {
                            let mins = (prog.position_secs / 60.0).floor() as u64;
                            let secs = (prog.position_secs % 60.0).floor() as u64;
                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "📺 Resume Stream ({mins:02}:{secs:02})"
                                    ))
                                    .color(egui::Color32::from_rgb(255, 100, 100))
                                    .strong(),
                                )
                                .clicked()
                            {
                                action = Some(PendingAction::StreamVideo {
                                    video_id: video.id.clone(),
                                    start_secs: Some(prog.position_secs),
                                });
                            }
                            if ui
                                .button(
                                    egui::RichText::new("↺ Stream Start")
                                        .color(egui::Color32::from_rgb(255, 180, 180)),
                                )
                                .clicked()
                            {
                                action = Some(PendingAction::StreamVideo {
                                    video_id: video.id.clone(),
                                    start_secs: Some(0.0),
                                });
                            }
                        } else if ui
                            .button(
                                egui::RichText::new("📺 Stream Video")
                                    .color(egui::Color32::from_rgb(255, 100, 100))
                                    .strong(),
                            )
                            .clicked()
                        {
                            action = Some(PendingAction::StreamVideo {
                                video_id: video.id.clone(),
                                start_secs: None,
                            });
                        }
                    } else if ui
                        .button(
                            egui::RichText::new("📺 Stream Video")
                                .color(egui::Color32::from_rgb(255, 100, 100))
                                .strong(),
                        )
                        .clicked()
                    {
                        action = Some(PendingAction::StreamVideo {
                            video_id: video.id.clone(),
                            start_secs: None,
                        });
                    }

                    if ui
                        .button(
                            egui::RichText::new("🌐 Watch in Browser")
                                .color(egui::Color32::from_rgb(100, 180, 255))
                                .strong(),
                        )
                        .clicked()
                    {
                        action = Some(PendingAction::OpenInBrowser {
                            url: format!("https://www.youtube.com/watch?v={}", video.id),
                        });
                    }
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(15.0);

                ui.label("Rate video:");
                if ui.button("👍 Like").clicked() {
                    action = Some(PendingAction::RateVideo {
                        video_id: video.id.clone(),
                        rating: "like".to_string(),
                    });
                }
                if ui.button("👎 Dislike").clicked() {
                    action = Some(PendingAction::RateVideo {
                        video_id: video.id.clone(),
                        rating: "dislike".to_string(),
                    });
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("📂 Add to Playlist:");
                match playlists {
                    None => {
                        ui.spinner();
                    }
                    Some(Err(_)) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            "Failed to load playlists.",
                        );
                    }
                    Some(Ok(items)) => {
                        egui::ComboBox::from_id_source("add_to_playlist_cb")
                            .selected_text("Select Playlist...")
                            .show_ui(ui, |ui| {
                                for playlist in items.iter() {
                                    if ui.selectable_label(false, &playlist.title).clicked() {
                                        action = Some(PendingAction::AddToPlaylist {
                                            playlist_id: playlist.id.clone(),
                                            playlist_title: playlist.title.clone(),
                                            video_id: video.id.clone(),
                                        });
                                    }
                                }
                            });
                    }
                }

                if let Some(status) = playlist_action_status {
                    ui.add_space(10.0);
                    match status {
                        Ok(msg) => {
                            ui.colored_label(egui::Color32::from_rgb(100, 255, 100), msg);
                        }
                        Err(err) => {
                            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                        }
                    }
                }
            });
        });
    });

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(10.0);

    ui.heading("💬 Comments");
    ui.add_space(5.0);

    // Comment submission form
    ui.horizontal(|ui| {
        ui.label("Write a comment:");
        let response = ui.text_edit_singleline(comment_input);
        let submit = ui.button("💬 Post Comment").clicked()
            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if submit {
            let text = comment_input.trim().to_string();
            if !text.is_empty() {
                action = Some(PendingAction::PostComment {
                    video_id: video.id.clone(),
                    text,
                });
                comment_input.clear();
            }
        }
    });
    ui.add_space(10.0);

    match comments {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.spinner();
                ui.label("Loading comments...");
            });
        }
        Some(Err(err_msg)) => {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Failed to load comments",
                );
                ui.add_space(5.0);
                ui.label(err_msg);
                ui.add_space(10.0);
                if ui.button("Retry").clicked() {
                    action = Some(PendingAction::RetryVideoDetails {
                        video: video.clone(),
                    });
                }
            });
        }
        Some(Ok(items)) => {
            if items.is_empty() {
                ui.label("No comments found on this video.");
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for comment in items.iter() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&comment.author_name)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(200, 200, 210)),
                                            );
                                            ui.add_space(10.0);
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Likes: {}",
                                                    comment.like_count
                                                ))
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(140, 140, 150)),
                                            );
                                        });
                                        ui.add_space(4.0);
                                        ui.label(&comment.text_display);
                                    });
                                    ui.add_space(5.0);
                                });
                            });
                        }

                        if loading_more_comments {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading more comments...");
                            });
                        } else if let Some(next_token) = comments_next_page_token {
                            ui.add_space(10.0);
                            if ui.button("💬 Load More Comments").clicked() {
                                action = Some(PendingAction::LoadMoreComments {
                                    video_id: video.id.clone(),
                                    page_token: next_token.clone(),
                                });
                            }
                        }
                    });
            }
        }
    }

    action
}
