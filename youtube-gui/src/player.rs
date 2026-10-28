//! # Background Audio Playback Worker
//!
//! Manages the Rodio audio mixer thread loop, device output initialization,
//! playback control commands, and UI repaint triggers upon track completion.

use crate::types::{AppState, PlayerCommand};
use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::append_to_log;

/// Spawn the background Rodio audio thread for local audio decoding and playback.
pub fn spawn_audio_worker(
    state: Arc<Mutex<AppState>>,
    audio_rx: std::sync::mpsc::Receiver<PlayerCommand>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let mut stream_opt: Option<rodio::MixerDeviceSink> = None;
        let mut sink_opt: Option<rodio::Player> = None;
        let mut has_active_track = false;
        let mut last_save = std::time::Instant::now();

        loop {
            let cmd_opt = audio_rx.recv_timeout(std::time::Duration::from_millis(200));
            match cmd_opt {
                Ok(cmd) => {
                    match cmd {
                        PlayerCommand::Play {
                            path,
                            title,
                            video_id,
                            start_secs,
                        } => {
                            if let Some(sink) = &sink_opt {
                                sink.stop();
                            }
                            has_active_track = false;

                            if stream_opt.is_none() {
                                match rodio::DeviceSinkBuilder::open_default_sink() {
                                    Ok(stream) => {
                                        let sink = rodio::Player::connect_new(stream.mixer());
                                        stream_opt = Some(stream);
                                        sink_opt = Some(sink);
                                    }
                                    Err(e) => {
                                        let mut s = state.lock().unwrap();
                                        append_to_log(
                                            &s.log_file,
                                            "ERROR",
                                            &format!("Failed to open default audio device: {e}"),
                                        );
                                        s.set_toast(
                                            format!("Could not open audio output device: {e}"),
                                            true,
                                        );
                                        s.player_state.playing = false;
                                        s.player_state.buffering = false;
                                        s.player_state.current_title.clear();
                                    }
                                }
                            }

                            if let Some(sink) = &sink_opt {
                                match std::fs::File::open(&path) {
                                    Ok(file) => {
                                        match rodio::Decoder::new(std::io::BufReader::new(file)) {
                                            Ok(source) => {
                                                use rodio::Source;
                                                let duration_secs = source
                                                    .total_duration()
                                                    .map(|d| d.as_secs_f32())
                                                    .unwrap_or(0.0);
                                                let mut s = state.lock().unwrap();
                                                sink.set_volume(s.player_state.volume);
                                                sink.append(source);
                                                sink.play();
                                                has_active_track = true;
                                                s.player_state.current_title = title.clone();
                                                s.player_state.current_video_id = video_id;
                                                s.player_state.playing = true;
                                                s.player_state.buffering = false;
                                                s.player_state.duration_secs = duration_secs;
                                                s.player_state.position_secs = 0.0;
                                                s.set_toast(format!("Playing: {title}"), false);

                                                if let Some(start) = start_secs {
                                                    if start > 1.0
                                                        && (duration_secs == 0.0
                                                            || start < duration_secs - 3.0)
                                                    {
                                                        let _ = sink.try_seek(
                                                            std::time::Duration::from_secs_f32(
                                                                start,
                                                            ),
                                                        );
                                                        s.player_state.position_secs = start;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let mut s = state.lock().unwrap();
                                                append_to_log(
                                                    &s.log_file,
                                                    "ERROR",
                                                    &format!(
                                                        "Failed to decode audio file {path:?}: {e}"
                                                    ),
                                                );
                                                s.set_toast(
                                                    format!("Failed to decode audio stream: {e}"),
                                                    true,
                                                );
                                                s.player_state.playing = false;
                                                s.player_state.buffering = false;
                                                s.player_state.current_title.clear();
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let mut s = state.lock().unwrap();
                                        append_to_log(
                                            &s.log_file,
                                            "ERROR",
                                            &format!("Failed to open audio file {path:?}: {e}"),
                                        );
                                        s.set_toast(
                                            format!("Failed to open audio file: {e}"),
                                            true,
                                        );
                                        s.player_state.playing = false;
                                        s.player_state.buffering = false;
                                        s.player_state.current_title.clear();
                                    }
                                }
                            }
                        }
                        PlayerCommand::PlayBytes {
                            data,
                            title,
                            video_id,
                            start_secs,
                        } => {
                            if let Some(sink) = &sink_opt {
                                sink.stop();
                            }
                            has_active_track = false;

                            if stream_opt.is_none() {
                                match rodio::DeviceSinkBuilder::open_default_sink() {
                                    Ok(stream) => {
                                        let sink = rodio::Player::connect_new(stream.mixer());
                                        stream_opt = Some(stream);
                                        sink_opt = Some(sink);
                                    }
                                    Err(e) => {
                                        let mut s = state.lock().unwrap();
                                        append_to_log(
                                            &s.log_file,
                                            "ERROR",
                                            &format!("Failed to open default audio device: {e}"),
                                        );
                                        s.set_toast(
                                            format!("Could not open audio output device: {e}"),
                                            true,
                                        );
                                        s.player_state.playing = false;
                                        s.player_state.buffering = false;
                                        s.player_state.current_title.clear();
                                    }
                                }
                            }

                            if let Some(sink) = &sink_opt {
                                match rodio::Decoder::new(std::io::Cursor::new(data)) {
                                    Ok(source) => {
                                        use rodio::Source;
                                        let duration_secs = source
                                            .total_duration()
                                            .map(|d| d.as_secs_f32())
                                            .unwrap_or(0.0);
                                        let mut s = state.lock().unwrap();
                                        sink.set_volume(s.player_state.volume);
                                        sink.append(source);
                                        sink.play();
                                        has_active_track = true;
                                        s.player_state.current_title = title.clone();
                                        s.player_state.current_video_id = video_id;
                                        s.player_state.playing = true;
                                        s.player_state.buffering = false;
                                        s.player_state.duration_secs = duration_secs;
                                        s.player_state.position_secs = 0.0;
                                        s.set_toast(format!("Playing: {title}"), false);

                                        if let Some(start) = start_secs {
                                            if start > 1.0
                                                && (duration_secs == 0.0
                                                    || start < duration_secs - 3.0)
                                            {
                                                let _ = sink.try_seek(
                                                    std::time::Duration::from_secs_f32(start),
                                                );
                                                s.player_state.position_secs = start;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let mut s = state.lock().unwrap();
                                        append_to_log(
                                            &s.log_file,
                                            "ERROR",
                                            &format!("Failed to decode audio bytes: {e}"),
                                        );
                                        s.set_toast(
                                            format!("Failed to decode audio stream: {e}"),
                                            true,
                                        );
                                        s.player_state.playing = false;
                                        s.player_state.buffering = false;
                                        s.player_state.current_title.clear();
                                    }
                                }
                            }
                        }
                        PlayerCommand::Pause => {
                            if let Some(sink) = &sink_opt {
                                sink.pause();
                                let mut s = state.lock().unwrap();
                                s.player_state.playing = false;
                                s.save_playback_positions();
                            }
                        }
                        PlayerCommand::Resume => {
                            if let Some(sink) = &sink_opt {
                                sink.play();
                                let mut s = state.lock().unwrap();
                                s.player_state.playing = true;
                            }
                        }
                        PlayerCommand::Stop => {
                            has_active_track = false;
                            if let Some(sink) = &sink_opt {
                                sink.stop();
                            }
                            let mut s = state.lock().unwrap();
                            if let Some(token) = s.audio_cancel_token.take() {
                                token.cancel();
                            }
                            s.player_state.playing = false;
                            s.player_state.buffering = false;
                            if let Some(vid) = s.player_state.current_video_id.take() {
                                let pos = s.player_state.position_secs;
                                let dur = s.player_state.duration_secs;
                                if dur > 0.0 && pos >= dur - 5.0 {
                                    s.playback_positions.remove(&vid);
                                } else if pos > 5.0 {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                    s.playback_positions.insert(
                                        vid,
                                        youtube_client_lib::utils::PlaybackProgress {
                                            position_secs: pos,
                                            duration_secs: dur,
                                            updated_at: now,
                                        },
                                    );
                                }
                                s.save_playback_positions();
                            }
                            s.player_state.current_title = String::new();
                            s.player_state.position_secs = 0.0;
                            s.player_state.duration_secs = 0.0;
                        }
                        PlayerCommand::SetVolume(vol) => {
                            if let Some(sink) = &sink_opt {
                                sink.set_volume(vol);
                            }
                            let mut s = state.lock().unwrap();
                            s.player_state.volume = vol;
                        }
                        PlayerCommand::Seek(dest) => {
                            if let Some(sink) = &sink_opt {
                                let _ = sink.try_seek(dest);
                                let mut s = state.lock().unwrap();
                                s.player_state.position_secs = dest.as_secs_f32();
                            }
                        }
                        PlayerCommand::Skip(secs) => {
                            if let Some(sink) = &sink_opt {
                                let cur = sink.get_pos();
                                let new_pos = if secs >= 0 {
                                    cur + std::time::Duration::from_secs(secs as u64)
                                } else {
                                    cur.saturating_sub(std::time::Duration::from_secs(
                                        (-secs) as u64,
                                    ))
                                };
                                let _ = sink.try_seek(new_pos);
                                let mut s = state.lock().unwrap();
                                s.player_state.position_secs = new_pos.as_secs_f32();
                            }
                        }
                    }
                    ctx.request_repaint();
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(sink) = &sink_opt {
                        if has_active_track && sink.empty() {
                            has_active_track = false;
                            let mut updated = false;
                            {
                                let mut s = state.lock().unwrap();
                                if !s.player_state.buffering {
                                    s.player_state.playing = false;
                                    if let Some(vid) = s.player_state.current_video_id.take() {
                                        s.playback_positions.remove(&vid);
                                        s.save_playback_positions();
                                    }
                                    s.player_state.current_title = String::new();
                                    s.player_state.position_secs = 0.0;
                                    s.player_state.duration_secs = 0.0;
                                    updated = true;
                                }
                            }
                            if updated {
                                ctx.request_repaint();
                            }
                        } else if has_active_track {
                            let pos = sink.get_pos().as_secs_f32();
                            let mut s = state.lock().unwrap();
                            if s.player_state.playing && !s.player_state.buffering {
                                s.player_state.position_secs = pos;
                                let dur = s.player_state.duration_secs;
                                if let Some(vid) = s.player_state.current_video_id.clone() {
                                    if dur > 0.0 && pos >= dur - 5.0 {
                                        s.playback_positions.remove(&vid);
                                    } else if pos > 5.0 {
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                        s.playback_positions.insert(
                                            vid,
                                            youtube_client_lib::utils::PlaybackProgress {
                                                position_secs: pos,
                                                duration_secs: dur,
                                                updated_at: now,
                                            },
                                        );
                                    }
                                    if last_save.elapsed() > std::time::Duration::from_secs(3) {
                                        last_save = std::time::Instant::now();
                                        s.save_playback_positions();
                                    }
                                }
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });
}
