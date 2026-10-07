//! # MPV IPC Client (Named Pipes on Windows / Unix Domain Sockets on Unix)
//!
//! Provides bidirectional remote control over running MPV instances, allowing
//! play, pause, seek, volume control, and playback position tracking.

use crate::error::{Result, YoutubeError};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

/// An IPC client for sending commands and querying properties from a running MPV player.
pub struct MpvIpcClient {
    pipe_path: String,
}

impl MpvIpcClient {
    /// Create a new IPC client targeting the specified named pipe or UNIX domain socket.
    pub fn new(pipe_path: impl Into<String>) -> Self {
        Self {
            pipe_path: pipe_path.into(),
        }
    }

    /// Generate a platform-appropriate default IPC socket/pipe path for an instance ID.
    pub fn generate_pipe_path(instance_id: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\pipe\mpv-ipc-{instance_id}")
        }
        #[cfg(not(windows))]
        {
            format!("/tmp/mpv-ipc-{instance_id}.sock")
        }
    }

    /// Returns the target IPC pipe path.
    pub fn pipe_path(&self) -> &str {
        &self.pipe_path
    }

    /// Send a command array to MPV and parse the JSON response.
    pub fn send_command(&self, command: &[&str]) -> Result<Value> {
        let cmd_obj = json!({
            "command": command
        });
        let mut line = cmd_obj.to_string();
        line.push('\n');

        #[cfg(windows)]
        {
            use std::fs::OpenOptions;
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.pipe_path)
                .map_err(|e| YoutubeError::Media(format!("Failed to connect to MPV IPC pipe: {e}")))?;

            file.write_all(line.as_bytes())
                .map_err(|e| YoutubeError::Media(format!("Failed to write to MPV IPC: {e}")))?;

            let mut reader = BufReader::new(file);
            let mut resp_line = String::new();
            reader
                .read_line(&mut resp_line)
                .map_err(|e| YoutubeError::Media(format!("Failed to read MPV IPC response: {e}")))?;

            let val: Value = serde_json::from_str(&resp_line)
                .map_err(|e| YoutubeError::Media(format!("Invalid MPV JSON response: {e}")))?;
            Ok(val)
        }

        #[cfg(not(windows))]
        {
            use std::os::unix::net::UnixStream;
            let mut stream = UnixStream::connect(&self.pipe_path)
                .map_err(|e| YoutubeError::Media(format!("Failed to connect to MPV IPC socket: {e}")))?;

            stream.write_all(line.as_bytes())
                .map_err(|e| YoutubeError::Media(format!("Failed to write to MPV IPC: {e}")))?;

            let mut reader = BufReader::new(stream);
            let mut resp_line = String::new();
            reader
                .read_line(&mut resp_line)
                .map_err(|e| YoutubeError::Media(format!("Failed to read MPV IPC response: {e}")))?;

            let val: Value = serde_json::from_str(&resp_line)
                .map_err(|e| YoutubeError::Media(format!("Invalid MPV JSON response: {e}")))?;
            Ok(val)
        }
    }

    /// Toggle playback pause state.
    pub fn toggle_pause(&self) -> Result<()> {
        let _ = self.send_command(&["cycle", "pause"])?;
        Ok(())
    }

    /// Set pause state directly.
    pub fn set_pause(&self, paused: bool) -> Result<()> {
        let val = if paused { "yes" } else { "no" };
        let _ = self.send_command(&["set_property", "pause", val])?;
        Ok(())
    }

    /// Seek relative seconds (e.g. +10.0 or -10.0).
    pub fn seek_relative(&self, seconds: f64) -> Result<()> {
        let sec_str = format!("{seconds}");
        let _ = self.send_command(&["seek", &sec_str, "relative"])?;
        Ok(())
    }

    /// Get current playback position in seconds if playing.
    pub fn get_time_pos(&self) -> Result<Option<f64>> {
        let res = self.send_command(&["get_property", "time-pos"])?;
        Ok(res.get("data").and_then(|d| d.as_f64()))
    }

    /// Set player volume (0.0 to 100.0).
    pub fn set_volume(&self, volume: f64) -> Result<()> {
        let vol_str = format!("{volume}");
        let _ = self.send_command(&["set_property", "volume", &vol_str])?;
        Ok(())
    }

    /// Quit MPV process cleanly.
    pub fn quit(&self) -> Result<()> {
        let _ = self.send_command(&["quit"])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pipe_path() {
        let path = MpvIpcClient::generate_pipe_path("test-123");
        assert!(path.contains("test-123"));
        #[cfg(windows)]
        assert!(path.starts_with(r"\\.\pipe\"));
        #[cfg(not(windows))]
        assert!(path.starts_with("/tmp/"));
    }
}
