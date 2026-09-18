use eframe::egui;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use youtube_client_lib::utils::DownloadStatus;
use youtube_client_lib::{Comment, Playlist, Subscription, Video, VideoDetails};

#[derive(Clone)]
pub struct Thumbnail {
    pub texture: Option<egui::TextureHandle>,
    pub loading: bool,
}

#[derive(Clone)]
pub enum View {
    Login,
    Subscriptions,
    NewVideos,
    ChannelVideos {
        channel_id: String,
        channel_title: String,
        channel_description: String,
        videos: Option<Result<Vec<Video>, String>>,
    },
    SearchResults {
        query: String,
        videos: Option<Result<Vec<Video>, String>>,
    },
    Playlists {
        playlists: Option<Result<Vec<Playlist>, String>>,
    },
    PlaylistVideos {
        playlist_id: String,
        playlist_title: String,
        videos: Option<Result<Vec<Video>, String>>,
    },
    VideoDetails {
        video: Video,
        details: Option<Result<VideoDetails, String>>,
        comments: Option<Result<Vec<Comment>, String>>,
    },
    About,
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub current_title: String,
    pub playing: bool,
    pub volume: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_title: String::new(),
            playing: false,
            volume: 1.0,
        }
    }
}

pub enum PlayerCommand {
    Play(PathBuf, String),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
}

pub struct AppState {
    pub subscriptions: Option<Result<Vec<Subscription>, String>>,
    pub new_videos: Option<Result<Vec<Video>, String>>,
    pub cleared_video_ids: HashSet<String>,
    pub thumbnails: HashMap<String, Thumbnail>,
    pub thumbnail_lru: VecDeque<String>,
    pub current_view: View,
    pub view_history: Vec<View>,
    pub logging_in: bool,
    pub login_error: Option<String>,
    pub downloads: HashMap<String, DownloadStatus>,
    pub player_state: PlayerState,
    pub playlists: Option<Result<Vec<Playlist>, String>>,
    pub playlist_action_status: Option<Result<String, String>>,
    pub downloads_dir: PathBuf,
    pub log_file: PathBuf,
    pub toast: Option<(String, std::time::Instant, bool)>,
}

/// Helper function to safely lock AppState with poison recovery.
pub fn lock_state(state: &Arc<Mutex<AppState>>) -> MutexGuard<'_, AppState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub const MAX_THUMBNAILS: usize = 150;

    pub fn set_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some((message.into(), std::time::Instant::now(), is_error));
    }

    pub fn insert_thumbnail(&mut self, key: String, thumbnail: Thumbnail) {
        if !self.thumbnails.contains_key(&key) {
            if self.thumbnails.len() >= Self::MAX_THUMBNAILS {
                if let Some(evict_key) = self.thumbnail_lru.pop_front() {
                    self.thumbnails.remove(&evict_key);
                }
            }
            self.thumbnail_lru.push_back(key.clone());
        }
        self.thumbnails.insert(key, thumbnail);
    }

    pub fn navigate_to(&mut self, new_view: View) {
        self.view_history.push(self.current_view.clone());
        self.current_view = new_view;
    }

    pub fn navigate_clear_history(&mut self, new_view: View) {
        self.view_history.clear();
        self.current_view = new_view;
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.view_history.pop() {
            self.current_view = prev;
        } else {
            self.current_view = View::Subscriptions;
        }
    }
}

#[allow(dead_code)]
pub enum PendingAction {
    None,
    SpawnDefaultLogin,
    SpawnLogin {
        id: String,
        secret: String,
    },
    RetrySubscriptions,
    GoToSubscriptions,
    GoToNewVideos,
    LoadNewVideos,
    ClearAllNewVideos,
    DismissNewVideo {
        video_id: String,
    },
    ResetClearedVideos,
    LoadChannel {
        id: String,
        title: String,
        description: String,
    },
    GoBack,
    RetryVideos {
        id: String,
        title: String,
        description: String,
    },
    SpawnDownload {
        video: Video,
        is_audio: bool,
    },
    PlayLocal {
        path: PathBuf,
        title: String,
    },
    StreamVideo {
        video_id: String,
    },
    OpenInBrowser {
        url: String,
    },
    Search {
        query: String,
    },
    RetrySearch {
        query: String,
    },
    LoadPlaylists,
    LoadPlaylistVideos {
        id: String,
        title: String,
    },
    RetryPlaylists,
    RetryPlaylistVideos {
        id: String,
        title: String,
    },
    LoadVideoDetails {
        video: Video,
    },
    RetryVideoDetails {
        video: Video,
    },
    Subscribe {
        channel_id: String,
    },
    Unsubscribe {
        subscription_id: String,
    },
    RateVideo {
        video_id: String,
        rating: String,
    },
    AddToPlaylist {
        playlist_id: String,
        playlist_title: String,
        video_id: String,
    },
    PostComment {
        video_id: String,
        text: String,
    },
    CreatePlaylist {
        title: String,
        description: Option<String>,
    },
    DeletePlaylist {
        playlist_id: String,
    },
    GoToAbout,
}
