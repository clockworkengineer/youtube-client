pub mod channel;
pub mod comments;
pub mod context;
pub mod details;
pub mod download;
pub mod history;
pub mod library;
pub mod login;
pub mod play;
pub mod playlist_manage;
pub mod playlists;
pub mod rate;
pub mod search;
pub mod subscription_manage;
pub mod subscriptions;
pub mod videos;

pub use channel::execute_channel;
pub use comments::{execute_comment_post, execute_comments};
pub use context::CliContext;
pub use details::execute_details;
pub use download::execute_download;
pub use history::execute_history;
pub use library::execute_library;
pub use login::execute_login;
pub use play::execute_play;
pub use playlist_manage::{
    execute_playlist_create, execute_playlist_delete, execute_playlist_remove_video,
    execute_playlist_update,
};
pub use playlists::execute_playlists;
pub use rate::execute_rate;
pub use search::execute_search;
pub use subscription_manage::{
    execute_export_subscriptions, execute_import_subscriptions, execute_subscribe,
    execute_unsubscribe,
};
pub use subscriptions::execute_subscriptions;
pub use videos::execute_videos;
