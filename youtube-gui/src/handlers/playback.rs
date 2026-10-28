use eframe::egui;
use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::download::{
    DownloadFormat, DownloadOptions, DownloadStage, download_video_with_options,
};
use youtube_client_lib::launch_external_player_with_options;
use youtube_client_lib::utils::append_to_log;

use crate::types::{AppState, PendingAction, PlayerCommand, lock_state};

/// Handle media playback, streaming, and player launching actions (Single Responsibility Principle).
pub fn handle_playback_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    audio_tx: &Sender<PlayerCommand>,
    ctx: &egui::Context,
) -> bool {
    match action {
        PendingAction::PlayLocal {
            path,
            title,
            video_id,
            start_secs,
        } => {
            let is_audio = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                e.eq_ignore_ascii_case("mp3")
                    || e.eq_ignore_ascii_case("m4a")
                    || e.eq_ignore_ascii_case("wav")
                    || e.eq_ignore_ascii_case("ogg")
                    || e.eq_ignore_ascii_case("flac")
            });
            let log_file = state.lock().unwrap().log_file.clone();
            let vid = video_id
                .clone()
                .or_else(|| youtube_client_lib::utils::extract_video_id_from_path(path));

            if is_audio {
                append_to_log(&log_file, "INFO", &format!("Playing local audio: {path:?}"));
                let _ = audio_tx.send(PlayerCommand::Play {
                    path: path.clone(),
                    title: title.clone(),
                    video_id: vid,
                    start_secs: *start_secs,
                });
            } else {
                append_to_log(&log_file, "INFO", &format!("Opening local video: {path:?}"));
                if let Err(e) = launch_external_player_with_options(
                    path.as_os_str(),
                    Some(&log_file),
                    *start_secs,
                ) {
                    append_to_log(
                        &log_file,
                        "WARN",
                        &format!("{e} Falling back to default file opener."),
                    );
                    let _ = open::that(path);
                }
            }
            true
        }
        PendingAction::StreamVideo {
            video_id,
            start_secs,
        } => {
            let state_clone = state.clone();
            let vid = video_id.clone();
            let start = *start_secs;

            {
                let mut s = lock_state(state);
                s.set_toast("Opening stream in external media player...", false);
            }

            tokio::spawn(async move {
                let base_url = format!("https://www.youtube.com/watch?v={vid}");
                let browser_url = if let Some(s_sec) = start {
                    if s_sec > 1.0 {
                        format!("https://www.youtube.com/watch?v={vid}&t={s_sec:.0}s")
                    } else {
                        base_url.clone()
                    }
                } else {
                    base_url.clone()
                };

                let (log_file, cached_segments, known_dur) = {
                    let s = lock_state(&state_clone);
                    let dur = match &s.current_view {
                        crate::types::View::VideoDetails {
                            video,
                            details: Some(Ok(d)),
                            ..
                        } if video.id == vid => d.duration_seconds as f64,
                        _ => s
                            .playback_positions
                            .get(&vid)
                            .map(|p| p.duration_secs as f64)
                            .unwrap_or(0.0),
                    };
                    (
                        s.log_file.clone(),
                        s.sponsor_segments.get(&vid).cloned(),
                        dur,
                    )
                };

                let config = youtube_client_lib::load_config();
                let sb_enabled = config.sponsorblock_enabled.unwrap_or(true);

                let mut target_uri = browser_url.clone();

                if sb_enabled {
                    let segments = if let Some(segs) = cached_segments {
                        segs
                    } else {
                        let http = reqwest::Client::new();
                        let cat_refs: Option<Vec<&str>> = config
                            .sponsorblock_categories
                            .as_ref()
                            .map(|v| v.iter().map(|s| s.as_str()).collect());
                        let fetched = youtube_client_lib::sponsorblock::fetch_skip_segments(
                            &http,
                            &vid,
                            cat_refs.as_deref(),
                        )
                        .await
                        .unwrap_or_default();
                        if !fetched.is_empty() {
                            let mut s = lock_state(&state_clone);
                            s.sponsor_segments.insert(vid.clone(), fetched.clone());
                        }
                        fetched
                    };

                    if !segments.is_empty() {
                        let effective_dur = if known_dur > 0.0 {
                            known_dur
                        } else {
                            segments
                                .iter()
                                .map(|seg| seg.end_secs)
                                .fold(0.0_f64, f64::max)
                                + 3600.0
                        };
                        if let Some(edl) = youtube_client_lib::sponsorblock::build_mpv_edl(
                            &base_url,
                            effective_dur,
                            &segments,
                        ) {
                            let registry = youtube_client_lib::PlayerRegistry::with_defaults();
                            if registry.default_player().is_some_and(|p| {
                                p.id() == "mpv" || p.display_name().to_lowercase().contains("mpv")
                            }) {
                                append_to_log(
                                    &log_file,
                                    "INFO",
                                    &format!(
                                        "SponsorBlock active: launching MPV with EDL ({} segments)",
                                        segments.len()
                                    ),
                                );
                                target_uri = edl;
                            }
                        }
                    }
                }

                append_to_log(
                    &log_file,
                    "INFO",
                    &format!("Streaming video target: {target_uri}"),
                );

                if let Err(e) = launch_external_player_with_options(
                    std::ffi::OsStr::new(&target_uri),
                    Some(&log_file),
                    start,
                ) {
                    append_to_log(
                        &log_file,
                        "WARN",
                        &format!("{e} Falling back to default browser."),
                    );
                    let mut s = lock_state(&state_clone);
                    s.set_toast("Media player not found; opening in browser", true);
                    let _ = open::that(&browser_url);
                }
            });
            true
        }
        PendingAction::StreamAudioDirect {
            video_id,
            title,
            start_secs,
        } => {
            let state_clone = state.clone();
            let audio_tx_clone = audio_tx.clone();
            let ctx_clone = ctx.clone();
            let vid = video_id.clone();
            let t = title.clone();
            let start = *start_secs;

            let cancel_token = tokio_util::sync::CancellationToken::new();
            let log_file = {
                let mut s = lock_state(state);
                if let Some(old_token) = s.audio_cancel_token.take() {
                    old_token.cancel();
                }
                s.audio_cancel_token = Some(cancel_token.clone());
                s.set_toast(format!("Buffering audio: {title}..."), false);
                s.player_state.current_title = format!("Buffering: {title}...");
                s.player_state.current_video_id = Some(vid.clone());
                s.player_state.playing = false;
                s.player_state.buffering = true;
                s.player_state.position_secs = 0.0;
                s.player_state.duration_secs = 0.0;
                s.log_file.clone()
            };
            ctx.request_repaint();

            tokio::spawn(async move {
                let cache_dir = std::env::temp_dir().join("youtube-client-audio");
                let _ = std::fs::create_dir_all(&cache_dir);

                let mp3_path = cache_dir.join(format!("{vid}.mp3"));
                let m4a_path = cache_dir.join(format!("{vid}.m4a"));

                // Reuse cached audio stream file if already downloaded and non-empty
                if mp3_path.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
                    if !cancel_token.is_cancelled() {
                        let _ = audio_tx_clone.send(PlayerCommand::Play {
                            path: mp3_path,
                            title: t,
                            video_id: Some(vid),
                            start_secs: start,
                        });
                    }
                    return;
                }
                if m4a_path.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
                    if !cancel_token.is_cancelled() {
                        let _ = audio_tx_clone.send(PlayerCommand::Play {
                            path: m4a_path,
                            title: t,
                            video_id: Some(vid),
                            start_secs: start,
                        });
                    }
                    return;
                }

                // Attempt 1: Download & extract MP3 (works with all YouTube audio codecs including Opus via ffmpeg)
                let mp3_opts = DownloadOptions::new(DownloadFormat::Mp3)
                    .with_log_file(log_file.clone())
                    .with_cancellation_token(cancel_token.clone());

                let state_prog = state_clone.clone();
                let ctx_prog = ctx_clone.clone();
                let title_prog = t.clone();
                let mp3_res =
                    download_video_with_options(&vid, &mp3_path, &mp3_opts, move |prog| {
                        let mut s = lock_state(&state_prog);
                        if s.player_state.buffering {
                            if prog.stage == DownloadStage::ExtractingAudio {
                                s.player_state.current_title =
                                    format!("Preparing audio: {title_prog}...");
                            } else if let Some(pct) = prog.percent {
                                s.player_state.current_title =
                                    format!("Buffering ({pct:.0}%): {title_prog}");
                            }
                            ctx_prog.request_repaint();
                        }
                    })
                    .await;

                if cancel_token.is_cancelled() {
                    return;
                }

                if mp3_res.is_ok() && mp3_path.exists() {
                    let _ = audio_tx_clone.send(PlayerCommand::Play {
                        path: mp3_path,
                        title: t,
                        video_id: Some(vid),
                        start_secs: start,
                    });
                    return;
                }

                // Attempt 2: Fallback to direct M4A/AAC stream (no ffmpeg required, supported by rodio's symphonia decoder)
                append_to_log(
                    &log_file,
                    "WARN",
                    &format!(
                        "MP3 audio extraction failed ({mp3_res:?}); falling back to direct M4A stream for {vid}"
                    ),
                );
                let m4a_opts = DownloadOptions::new(DownloadFormat::Custom(
                    "ba[ext=m4a]/b[ext=mp4]".to_string(),
                ))
                .with_log_file(log_file.clone())
                .with_cancellation_token(cancel_token.clone());

                let state_prog2 = state_clone.clone();
                let ctx_prog2 = ctx_clone.clone();
                let title_prog2 = t.clone();
                let m4a_res =
                    download_video_with_options(&vid, &m4a_path, &m4a_opts, move |prog| {
                        let mut s = lock_state(&state_prog2);
                        if s.player_state.buffering {
                            if let Some(pct) = prog.percent {
                                s.player_state.current_title =
                                    format!("Buffering ({pct:.0}%): {title_prog2}");
                            }
                            ctx_prog2.request_repaint();
                        }
                    })
                    .await;

                if cancel_token.is_cancelled() {
                    return;
                }

                match m4a_res {
                    Ok(()) if m4a_path.exists() => {
                        let _ = audio_tx_clone.send(PlayerCommand::Play {
                            path: m4a_path,
                            title: t,
                            video_id: Some(vid),
                            start_secs: start,
                        });
                    }
                    Ok(()) => {
                        let mut s = lock_state(&state_clone);
                        s.set_toast("Failed to stream audio: output file was not created", true);
                        s.player_state.playing = false;
                        s.player_state.buffering = false;
                        s.player_state.current_title.clear();
                        ctx_clone.request_repaint();
                    }
                    Err(e) => {
                        let err_msg = mp3_res
                            .err()
                            .map(|first_err| format!("{first_err}"))
                            .unwrap_or_else(|| format!("{e}"));
                        append_to_log(
                            &log_file,
                            "ERROR",
                            &format!("Audio streaming failed for {vid}: {err_msg}"),
                        );
                        let mut s = lock_state(&state_clone);
                        s.set_toast(format!("Failed to stream audio: {err_msg}"), true);
                        s.player_state.playing = false;
                        s.player_state.buffering = false;
                        s.player_state.current_title.clear();
                        ctx_clone.request_repaint();
                    }
                }
            });
            true
        }
        PendingAction::OpenInBrowser { url } => {
            let log_file = state.lock().unwrap().log_file.clone();
            append_to_log(&log_file, "INFO", &format!("Opening in web browser: {url}"));
            let mut s = lock_state(state);
            s.set_toast("Opening in web browser...", false);
            let _ = open::that(url);
            true
        }
        _ => false,
    }
}
