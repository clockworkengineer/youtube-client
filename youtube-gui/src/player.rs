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

        loop {
            let cmd_opt = audio_rx.recv_timeout(std::time::Duration::from_millis(200));
            match cmd_opt {
                Ok(cmd) => {
                    match cmd {
                        PlayerCommand::Play(path, title) => {
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
                                        s.player_state.playing = true;
                                        s.player_state.duration_secs = duration_secs;
                                        s.player_state.position_secs = 0.0;
                                    }
                                }
                            }
                        }
                        PlayerCommand::Pause => {
                            if let Some(sink) = &sink_opt {
                                sink.pause();
                                let mut s = state.lock().unwrap();
                                s.player_state.playing = false;
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
