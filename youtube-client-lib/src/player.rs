//! # Extensible Media Player Abstraction & Registry (Open/Closed Principle)
//!
//! Provides the [`MediaPlayer`] trait and [`PlayerRegistry`], decoupling external media
//! player dispatching from hardcoded player strings and allowing new players to be added
//! without modifying existing code.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use crate::utils::append_to_log;

/// Options and environment flags passed to external media player launches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayOptions {
    /// Playback start offset in seconds.
    pub start_secs: Option<f32>,
    /// Path to a cookies file (e.g. Netscape cookies format).
    pub cookies_file: Option<PathBuf>,
    /// Browser name to extract cookies from (e.g. "chrome", "firefox").
    pub cookies_from_browser: Option<String>,
    /// Destination log file for capturing child process stdout/stderr.
    pub log_file: Option<PathBuf>,
    /// Optional window or media title.
    pub title: Option<String>,
}

impl PlayOptions {
    /// Create default play options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set playback start offset in seconds.
    pub fn with_start_secs(mut self, start_secs: Option<f32>) -> Self {
        self.start_secs = start_secs;
        self
    }

    /// Set cookies file path.
    pub fn with_cookies_file(mut self, cookies_file: Option<PathBuf>) -> Self {
        self.cookies_file = cookies_file;
        self
    }

    /// Set browser name for cookie extraction.
    pub fn with_cookies_from_browser(mut self, browser: Option<String>) -> Self {
        self.cookies_from_browser = browser;
        self
    }

    /// Set log file path.
    pub fn with_log_file(mut self, log_file: Option<PathBuf>) -> Self {
        self.log_file = log_file;
        self
    }

    /// Set window or media title.
    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }
}

/// Configure child process logging and window suppression flags.
pub fn configure_process_logging(cmd: &mut Command, log_file: Option<&Path>) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let log_file_path = log_file
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate::config::resolve_log_file_path(None));

    if let Some(parent) = log_file_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        if let Ok(file_err) = file.try_clone() {
            cmd.stdout(Stdio::from(file));
            cmd.stderr(Stdio::from(file_err));
        } else {
            cmd.stdout(Stdio::from(file));
            cmd.stderr(Stdio::null());
        }
    } else {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }

    log_file_path
}

/// Helper to search for an executable in the system PATH or candidate paths.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_file() {
        return Some(p.to_path_buf());
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(target_os = "windows")]
            {
                let candidate_exe = dir.join(format!("{name}.exe"));
                if candidate_exe.is_file() {
                    return Some(candidate_exe);
                }
            }
        }
    }
    None
}

/// Trait defining an external media player integration (Open/Closed Principle).
pub trait MediaPlayer: Send + Sync {
    /// Unique machine identifier for the player (e.g. `"mpv"`, `"vlc"`).
    fn id(&self) -> &str;

    /// Human-friendly display name for user interfaces.
    fn display_name(&self) -> &str;

    /// Returns `true` if the player binary is detected and runnable on the current machine.
    fn is_available(&self) -> bool;

    /// Launch playback for a media target (URL or file path) using the given options.
    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String>;
}

/// MPV media player implementation.
#[derive(Clone, Debug, Default)]
pub struct MpvPlayer {
    executable: Option<PathBuf>,
}

impl MpvPlayer {
    pub fn new() -> Self {
        let executable = find_executable("mpv");
        Self { executable }
    }
}

impl MediaPlayer for MpvPlayer {
    fn id(&self) -> &str {
        "mpv"
    }

    fn display_name(&self) -> &str {
        "MPV"
    }

    fn is_available(&self) -> bool {
        self.executable.is_some()
    }

    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String> {
        let exe = self
            .executable
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "mpv".to_string());

        let mut cmd = Command::new(&exe);
        cmd.arg("--no-terminal");
        cmd.arg("--save-position-on-quit");

        if let Some(start) = opts.start_secs {
            if start > 1.0 {
                cmd.arg(format!("--start={start:.1}"));
            }
        }

        if let Some(ref title) = opts.title {
            cmd.arg(format!("--force-media-title={title}"));
        }

        let target_str = target.to_string_lossy();
        let is_url = target_str.starts_with("http://") || target_str.starts_with("https://");

        if is_url {
            if let Some(ref cf) = opts.cookies_file {
                cmd.arg(format!(
                    "--ytdl-raw-options-append=cookies={}",
                    cf.display()
                ));
            }
            if let Some(ref cb) = opts.cookies_from_browser {
                cmd.arg(format!(
                    "--ytdl-raw-options-append=cookies-from-browser={cb}"
                ));
            }
        }

        configure_process_logging(&mut cmd, opts.log_file.as_deref());
        cmd.arg(target);

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn MPV ({exe}): {e}"))
    }
}

/// VLC media player implementation.
#[derive(Clone, Debug, Default)]
pub struct VlcPlayer {
    executable: Option<PathBuf>,
}

impl VlcPlayer {
    pub fn new() -> Self {
        let mut candidates = vec![PathBuf::from("vlc")];

        #[cfg(target_os = "windows")]
        {
            candidates.push(PathBuf::from("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe"));
            candidates.push(PathBuf::from(
                "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe",
            ));
        }

        #[cfg(target_os = "macos")]
        {
            candidates.push(PathBuf::from("/Applications/VLC.app/Contents/MacOS/VLC"));
        }

        let mut found = None;
        for c in candidates {
            if let Some(path) = find_executable(&c.to_string_lossy()) {
                found = Some(path);
                break;
            }
        }

        Self { executable: found }
    }
}

impl MediaPlayer for VlcPlayer {
    fn id(&self) -> &str {
        "vlc"
    }

    fn display_name(&self) -> &str {
        "VLC"
    }

    fn is_available(&self) -> bool {
        self.executable.is_some()
    }

    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String> {
        let exe = self
            .executable
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "vlc".to_string());

        let mut cmd = Command::new(&exe);

        if let Some(start) = opts.start_secs {
            if start > 1.0 {
                cmd.arg(format!("--start-time={start:.0}"));
            }
        }

        if let Some(ref title) = opts.title {
            cmd.arg(format!("--meta-title={title}"));
        }

        configure_process_logging(&mut cmd, opts.log_file.as_deref());
        cmd.arg(target);

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn VLC ({exe}): {e}"))
    }
}

/// IINA media player for macOS.
#[derive(Clone, Debug, Default)]
pub struct IinaPlayer {
    executable: Option<PathBuf>,
}

impl IinaPlayer {
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut found = find_executable("iina");
        #[cfg(target_os = "macos")]
        if found.is_none() {
            let app_path = Path::new("/Applications/IINA.app/Contents/MacOS/IINA");
            if app_path.is_file() {
                found = Some(app_path.to_path_buf());
            }
        }
        Self { executable: found }
    }
}

impl MediaPlayer for IinaPlayer {
    fn id(&self) -> &str {
        "iina"
    }

    fn display_name(&self) -> &str {
        "IINA"
    }

    fn is_available(&self) -> bool {
        self.executable.is_some()
    }

    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String> {
        let exe = self
            .executable
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "iina".to_string());

        let mut cmd = Command::new(&exe);

        if let Some(start) = opts.start_secs {
            if start > 1.0 {
                cmd.arg(format!("--mpv-start={start:.1}"));
            }
        }

        configure_process_logging(&mut cmd, opts.log_file.as_deref());
        cmd.arg(target);

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn IINA ({exe}): {e}"))
    }
}

/// Custom user-specified executable or script player.
#[derive(Clone, Debug)]
pub struct CustomExecutablePlayer {
    executable: String,
}

impl CustomExecutablePlayer {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl MediaPlayer for CustomExecutablePlayer {
    fn id(&self) -> &str {
        "custom"
    }

    fn display_name(&self) -> &str {
        "Custom Player"
    }

    fn is_available(&self) -> bool {
        !self.executable.is_empty()
            && (Path::new(&self.executable).is_file()
                || find_executable(&self.executable).is_some())
    }

    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String> {
        let mut cmd = Command::new(&self.executable);

        let lower = self.executable.to_lowercase();
        if lower.contains("mpv") {
            cmd.arg("--no-terminal");
            cmd.arg("--save-position-on-quit");
            if let Some(start) = opts.start_secs {
                if start > 1.0 {
                    cmd.arg(format!("--start={start:.1}"));
                }
            }
            if let Some(ref title) = opts.title {
                cmd.arg(format!("--force-media-title={title}"));
            }
        } else if lower.contains("vlc") {
            if let Some(start) = opts.start_secs {
                if start > 1.0 {
                    cmd.arg(format!("--start-time={start:.0}"));
                }
            }
            if let Some(ref title) = opts.title {
                cmd.arg(format!("--meta-title={title}"));
            }
        }

        configure_process_logging(&mut cmd, opts.log_file.as_deref());
        cmd.arg(target);

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn custom player ({}): {e}", self.executable))
    }
}

/// System default file/URL handler (fallback via standard desktop launcher).
#[derive(Clone, Debug, Default)]
pub struct SystemDefaultPlayer;

impl MediaPlayer for SystemDefaultPlayer {
    fn id(&self) -> &str {
        "system"
    }

    fn display_name(&self) -> &str {
        "System Default"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<Child, String> {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg("start").arg("").arg(target);
            configure_process_logging(&mut cmd, opts.log_file.as_deref());
            cmd.spawn()
                .map_err(|e| format!("Failed to launch default Windows handler: {e}"))
        }

        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("open");
            cmd.arg(target);
            configure_process_logging(&mut cmd, opts.log_file.as_deref());
            cmd.spawn()
                .map_err(|e| format!("Failed to launch macOS 'open': {e}"))
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let mut cmd = Command::new("xdg-open");
            cmd.arg(target);
            configure_process_logging(&mut cmd, opts.log_file.as_deref());
            cmd.spawn()
                .map_err(|e| format!("Failed to launch 'xdg-open': {e}"))
        }
    }
}

/// Registry of available media players allowing dynamic player discovery, selection,
/// and prioritized fallback dispatching without code modification (Open/Closed Principle).
#[derive(Clone, Default)]
pub struct PlayerRegistry {
    players: Vec<Arc<dyn MediaPlayer>>,
}

impl PlayerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }

    /// Construct a registry populated with default system players in priority order.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // 1. Check user-configured player from config.json
        if let Some(user_player) = crate::utils::get_configured_player_path() {
            registry.register(Arc::new(CustomExecutablePlayer::new(user_player)));
        }

        // 2. Add standard known players
        registry.register(Arc::new(MpvPlayer::new()));
        registry.register(Arc::new(VlcPlayer::new()));

        #[cfg(target_os = "macos")]
        registry.register(Arc::new(IinaPlayer::new()));

        // 3. Fallback to system default desktop handler
        registry.register(Arc::new(SystemDefaultPlayer));

        registry
    }

    /// Register a new media player implementation.
    pub fn register(&mut self, player: Arc<dyn MediaPlayer>) {
        self.players.push(player);
    }

    /// Retrieve a player by its unique ID.
    pub fn get_player(&self, id: &str) -> Option<Arc<dyn MediaPlayer>> {
        self.players.iter().find(|p| p.id() == id).cloned()
    }

    /// Returns all registered players.
    pub fn all_players(&self) -> &[Arc<dyn MediaPlayer>] {
        &self.players
    }

    /// Returns only the registered players that are installed and available on this machine.
    pub fn available_players(&self) -> Vec<Arc<dyn MediaPlayer>> {
        self.players
            .iter()
            .filter(|p| p.is_available())
            .cloned()
            .collect()
    }

    /// Returns the highest-priority available player.
    pub fn default_player(&self) -> Option<Arc<dyn MediaPlayer>> {
        self.players.iter().find(|p| p.is_available()).cloned()
    }

    /// Launch playback by iterating registered players in priority order until one succeeds.
    pub fn launch(&self, target: &OsStr, opts: &PlayOptions) -> Result<(), String> {
        let target_str = target.to_string_lossy();
        let log_file_path = opts
            .log_file
            .clone()
            .unwrap_or_else(|| crate::config::resolve_log_file_path(None));

        for player in &self.players {
            if !player.is_available() {
                continue;
            }

            match player.launch(target, opts) {
                Ok(_) => {
                    append_to_log(
                        &log_file_path,
                        "INFO",
                        &format!(
                            "Launched media player '{}' for target: {target_str}",
                            player.display_name()
                        ),
                    );
                    return Ok(());
                }
                Err(err) => {
                    append_to_log(
                        &log_file_path,
                        "WARN",
                        &format!("Player '{}' failed to launch: {err}", player.display_name()),
                    );
                }
            }
        }

        append_to_log(
            &log_file_path,
            "ERROR",
            &format!("No media players succeeded for target: {target_str}"),
        );
        Err("No media players succeeded.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPlayer {
        id: &'static str,
        name: &'static str,
        available: bool,
    }

    impl MediaPlayer for MockPlayer {
        fn id(&self) -> &str {
            self.id
        }

        fn display_name(&self) -> &str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn launch(&self, _target: &OsStr, _opts: &PlayOptions) -> Result<Child, String> {
            if self.available {
                // Return dummy child process on Windows or Unix
                #[cfg(target_os = "windows")]
                let child = Command::new("cmd").arg("/C").arg("exit 0").spawn().unwrap();
                #[cfg(not(target_os = "windows"))]
                let child = Command::new("true").spawn().unwrap();
                Ok(child)
            } else {
                Err("Player not installed".to_string())
            }
        }
    }

    #[test]
    fn test_player_registry_discovery() {
        let mut registry = PlayerRegistry::new();
        registry.register(Arc::new(MockPlayer {
            id: "fake_inactive",
            name: "Inactive Player",
            available: false,
        }));
        registry.register(Arc::new(MockPlayer {
            id: "fake_active",
            name: "Active Player",
            available: true,
        }));

        assert_eq!(registry.all_players().len(), 2);
        let avail = registry.available_players();
        assert_eq!(avail.len(), 1);
        assert_eq!(avail[0].id(), "fake_active");

        let default_player = registry.default_player().unwrap();
        assert_eq!(default_player.id(), "fake_active");

        let res = registry.launch(OsStr::new("https://example.com/test"), &PlayOptions::new());
        assert!(res.is_ok());
    }

    #[test]
    fn test_play_options_fluent_builder() {
        let opts = PlayOptions::new()
            .with_start_secs(Some(42.5))
            .with_title(Some("Test Video".to_string()))
            .with_cookies_from_browser(Some("firefox".to_string()));

        assert_eq!(opts.start_secs, Some(42.5));
        assert_eq!(opts.title, Some("Test Video".to_string()));
        assert_eq!(opts.cookies_from_browser, Some("firefox".to_string()));
    }
}
