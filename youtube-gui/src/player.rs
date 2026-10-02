//! # Background Audio Playback Worker
//!
//! Manages the Rodio audio mixer thread loop, device output initialization,
//! playback control commands, and UI repaint triggers upon track completion.

use crate::types::{AppState, PlayerCommand};
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Spawn the background Rodio audio thread for local audio decoding and playback.
pub fn spawn_audio_worker(
    state: Arc<Mutex<AppState>>,
    audio_rx: std::sync::mpsc::Receiver<PlayerCommand>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let mut stream_opt: Option<rodio::MixerDeviceSink> = None;
        let mut sink_opt: Option<rodio::Player> = None;
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
                            if stream_opt.is_none() {
                                if let Ok(stream) = rodio::DeviceSinkBuilder::open_default_sink() {
                                    let sink = rodio::Player::connect_new(stream.mixer());
                                    stream_opt = Some(stream);
                                    sink_opt = Some(sink);
                                }
                            }
                            if let Some(sink) = &sink_opt {
                                if let Ok(file) = std::fs::File::open(&path) {
                                    if let Ok(source) =
                                        rodio::Decoder::new(std::io::BufReader::new(file))
                                    {
                                        use rodio::Source;
                                        let duration_secs = source
                                            .total_duration()
                                            .map(|d| d.as_secs_f32())
                                            .unwrap_or(0.0);
                                        let mut s = state.lock().unwrap();
                                        sink.set_volume(s.player_state.volume);
                                        sink.append(source);
                                        sink.play();
                                        s.player_state.current_title = title;
                                        s.player_state.current_video_id = video_id;
                                        s.player_state.playing = true;
                                        s.player_state.duration_secs = duration_secs;
                                        s.player_state.position_secs = 0.0;

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
                            if let Some(sink) = &sink_opt {
                                sink.stop();
                                let mut s = state.lock().unwrap();
                                s.player_state.playing = false;
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
                        if sink.empty() {
                            let mut updated = false;
                            {
                                let mut s = state.lock().unwrap();
                                if s.player_state.playing {
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
                        } else {
                            let pos = sink.get_pos().as_secs_f32();
                            let mut s = state.lock().unwrap();
                            if s.player_state.playing {
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
