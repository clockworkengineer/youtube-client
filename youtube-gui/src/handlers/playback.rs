use std::sync::{Arc, Mutex, mpsc::Sender};
use youtube_client_lib::launch_external_player_with_options;
use youtube_client_lib::utils::append_to_log;

use crate::types::{AppState, PendingAction, PlayerCommand, lock_state};

/// Handle media playback, streaming, and player launching actions (Single Responsibility Principle).
pub fn handle_playback_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    audio_tx: &Sender<PlayerCommand>,
) -> bool {
    match action {
        PendingAction::PlayLocal {
            path,
            title,
            video_id,
            start_secs,
        } => {
            let is_mp3 = path.extension().map(|e| e == "mp3").unwrap_or(false);
            let log_file = state.lock().unwrap().log_file.clone();
            let vid = video_id
                .clone()
                .or_else(|| youtube_client_lib::utils::extract_video_id_from_path(path));

            if is_mp3 {
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
                            if registry
                                .default_player()
                                .is_some_and(|p| p.id() == "mpv" || p.display_name().to_lowercase().contains("mpv"))
                            {
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

                append_to_log(&log_file, "INFO", &format!("Streaming video target: {target_uri}"));

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
            let vid = video_id.clone();
            let t = title.clone();
            let start = *start_secs;

            {
                let mut s = lock_state(state);
                s.set_toast(format!("Buffering audio: {title}..."), false);
                s.player_state.current_title = format!("Buffering: {title}...");
                s.player_state.current_video_id = Some(vid.clone());
                s.player_state.playing = true;
            }

            tokio::spawn(async move {
                let url = format!("https://www.youtube.com/watch?v={vid}");
                let mut cmd = tokio::process::Command::new("yt-dlp");
                cmd.arg("-f")
                    .arg("bestaudio")
                    .arg("-o")
                    .arg("-")
                    .arg("--")
                    .arg(&url);

                #[cfg(windows)]
                cmd.creation_flags(0x08000000);

                match cmd
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(mut child) => {
                        use tokio::io::AsyncReadExt;
                        if let Some(mut stdout) = child.stdout.take() {
                            let mut buffer = Vec::new();
                            let mut temp = [0u8; 16384];
                            while let Ok(n) = stdout.read(&mut temp).await {
                                if n == 0 {
                                    break;
                                }
                                buffer.extend_from_slice(&temp[..n]);
                            }
                            let _ = child.wait().await;
                            if !buffer.is_empty() {
                                let _ = audio_tx_clone.send(PlayerCommand::PlayBytes {
                                    data: buffer,
                                    title: t,
                                    video_id: Some(vid),
                                    start_secs: start,
                                });
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let mut s = lock_state(&state_clone);
                        s.set_toast(format!("Failed to stream audio: {e}"), true);
                        s.player_state.playing = false;
                        s.player_state.current_title = String::new();
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
