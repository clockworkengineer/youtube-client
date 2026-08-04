use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use eframe::egui;
use youtube_client_lib::utils::DownloadStatus;
use youtube_client_lib::{Subscription, Video, Playlist, Comment};

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
        comments: Option<Result<Vec<Comment>, String>>,
    },
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub current_title: String,
    pub playing: bool,
}

pub enum PlayerCommand {
    Play(PathBuf, String),
    Pause,
    Resume,
    Stop,
}

pub struct AppState {
    pub subscriptions: Option<Result<Vec<Subscription>, String>>,
    pub new_videos: Option<Result<Vec<Video>, String>>,
    pub cleared_video_ids: HashSet<String>,
    pub thumbnails: HashMap<String, Thumbnail>,
    pub current_view: View,
    pub view_history: Vec<View>,
    pub logging_in: bool,
    pub login_error: Option<String>,
    pub downloads: HashMap<String, DownloadStatus>,
    pub player_state: PlayerState,
    pub playlists: Option<Result<Vec<Playlist>, String>>,
    pub playlist_action_status: Option<Result<String, String>>,
    pub downloads_dir: PathBuf,
}

impl AppState {
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
    SpawnLogin { id: String, secret: String },
    RetrySubscriptions,
    GoToSubscriptions,
    GoToNewVideos,
    LoadNewVideos,
    ClearAllNewVideos,
    DismissNewVideo { video_id: String },
    ResetClearedVideos,
    LoadChannel { id: String, title: String, description: String },
    GoBack,
    RetryVideos { id: String, title: String, description: String },
    SpawnDownload { video: Video, is_audio: bool },
    PlayLocal { path: PathBuf, title: String },
    StreamVideo { video_id: String },
    Search { query: String },
    RetrySearch { query: String },
    LoadPlaylists,
    LoadPlaylistVideos { id: String, title: String },
    RetryPlaylists,
    RetryPlaylistVideos { id: String, title: String },
    LoadVideoDetails { video: Video },
    RetryVideoDetails { video: Video },
    Subscribe { channel_id: String },
    Unsubscribe { subscription_id: String },
    RateVideo { video_id: String, rating: String },
    AddToPlaylist { playlist_id: String, playlist_title: String, video_id: String },
}
