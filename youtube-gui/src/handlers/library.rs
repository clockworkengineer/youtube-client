use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::save_string_set_to_file;

use crate::actions::{
    fetch_playlist_videos, fetch_search_results, fetch_videos, spawn_add_to_playlist,
    spawn_create_playlist, spawn_delete_playlist, spawn_fetch_comments, spawn_fetch_new_videos,
    spawn_fetch_playlists, spawn_fetch_subscriptions, spawn_post_comment, spawn_rate_video,
    spawn_subscribe, spawn_unsubscribe,
};
use crate::types::{AppState, PendingAction, View};

/// Handle YouTube media library, feed, playlist, comment, and subscription actions (Single Responsibility Principle).
pub fn handle_library_action(
    action: &PendingAction,
    state: &Arc<Mutex<AppState>>,
    ctx: &egui::Context,
) -> bool {
    match action {
        PendingAction::RetrySubscriptions => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.subscriptions = None;
            }
            spawn_fetch_subscriptions(state.clone(), ctx.clone());
            true
        }
        PendingAction::LoadNewVideos => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.new_videos = None;
                s_lock.current_view = View::NewVideos;
            }
            spawn_fetch_new_videos(state.clone(), ctx.clone());
            true
        }
        PendingAction::ClearAllNewVideos => {
            let mut s_lock = state.lock().unwrap();
            let ids_to_clear: Vec<String> = match &s_lock.new_videos {
                Some(Ok(vids)) => vids.iter().map(|v| v.id.clone()).collect(),
                _ => Vec::new(),
            };
            for id in ids_to_clear {
                s_lock.cleared_video_ids.insert(id);
            }
            let cleared_path = s_lock.cleared_videos_path.clone();
            let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
            s_lock.new_videos = Some(Ok(Vec::new()));
            true
        }
        PendingAction::DismissNewVideo { video_id } => {
            let mut s_lock = state.lock().unwrap();
            s_lock.cleared_video_ids.insert(video_id.clone());
            let cleared_path = s_lock.cleared_videos_path.clone();
            let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
            if let Some(Ok(ref mut vids)) = s_lock.new_videos {
                vids.retain(|v| v.id != *video_id);
            }
            true
        }
        PendingAction::ResetClearedVideos => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.cleared_video_ids.clear();
                let cleared_path = s_lock.cleared_videos_path.clone();
                let _ = save_string_set_to_file(&cleared_path, &s_lock.cleared_video_ids);
                s_lock.new_videos = None;
            }
            spawn_fetch_new_videos(state.clone(), ctx.clone());
            true
        }
        PendingAction::LoadChannel {
            id,
            title,
            description,
        } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.navigate_to(View::ChannelVideos {
                    channel_id: id.clone(),
                    channel_title: title.clone(),
                    channel_description: description.clone(),
                    videos: None,
                });
            }
            fetch_videos(ctx.clone(), state.clone(), id.clone(), title.clone());
            true
        }
        PendingAction::RetryVideos {
            id,
            title,
            description,
        } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.current_view = View::ChannelVideos {
                    channel_id: id.clone(),
                    channel_title: title.clone(),
                    channel_description: description.clone(),
                    videos: None,
                };
            }
            fetch_videos(ctx.clone(), state.clone(), id.clone(), title.clone());
            true
        }
        PendingAction::Search { query } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.navigate_to(View::SearchResults {
                    query: query.clone(),
                    videos: None,
                });
            }
            fetch_search_results(ctx.clone(), state.clone(), query.clone());
            true
        }
        PendingAction::RetrySearch { query } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.current_view = View::SearchResults {
                    query: query.clone(),
                    videos: None,
                };
            }
            fetch_search_results(ctx.clone(), state.clone(), query.clone());
            true
        }
        PendingAction::LoadPlaylists => {
            let cache = {
                let mut s_lock = state.lock().unwrap();
                let cached = s_lock.playlists.clone();
                s_lock.navigate_clear_history(View::Playlists {
                    playlists: cached.clone(),
                });
                cached
            };
            if cache.is_none() {
                spawn_fetch_playlists(state.clone(), ctx.clone());
            }
            true
        }
        PendingAction::RetryPlaylists => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.playlists = None;
                s_lock.current_view = View::Playlists { playlists: None };
            }
            spawn_fetch_playlists(state.clone(), ctx.clone());
            true
        }
        PendingAction::LoadPlaylistVideos { id, title } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.navigate_to(View::PlaylistVideos {
                    playlist_id: id.clone(),
                    playlist_title: title.clone(),
                    videos: None,
                });
            }
            fetch_playlist_videos(ctx.clone(), state.clone(), id.clone(), title.clone());
            true
        }
        PendingAction::RetryPlaylistVideos { id, title } => {
            {
                let mut s_lock = state.lock().unwrap();
                s_lock.current_view = View::PlaylistVideos {
                    playlist_id: id.clone(),
                    playlist_title: title.clone(),
                    videos: None,
                };
            }
            fetch_playlist_videos(ctx.clone(), state.clone(), id.clone(), title.clone());
            true
        }
        PendingAction::LoadVideoDetails { video } => {
            let cache = {
                let mut s_lock = state.lock().unwrap();
                s_lock.playlist_action_status = None;
                s_lock.navigate_to(View::VideoDetails {
                    video: video.clone(),
                    details: None,
                    comments: None,
                });
                s_lock.playlists.clone()
            };
            spawn_fetch_comments(state.clone(), ctx.clone(), video.clone());
            if cache.is_none() {
                spawn_fetch_playlists(state.clone(), ctx.clone());
            }
            true
        }
        PendingAction::RetryVideoDetails { video } => {
            let cache = {
                let mut s_lock = state.lock().unwrap();
                s_lock.playlist_action_status = None;
                s_lock.current_view = View::VideoDetails {
                    video: video.clone(),
                    details: None,
                    comments: None,
                };
                s_lock.playlists.clone()
            };
            spawn_fetch_comments(state.clone(), ctx.clone(), video.clone());
            if cache.is_none() {
                spawn_fetch_playlists(state.clone(), ctx.clone());
            }
            true
        }
        PendingAction::PostComment { video_id, text } => {
            spawn_post_comment(state.clone(), ctx.clone(), video_id.clone(), text.clone());
            true
        }
        PendingAction::CreatePlaylist { title, description } => {
            spawn_create_playlist(
                state.clone(),
                ctx.clone(),
                title.clone(),
                description.clone(),
            );
            true
        }
        PendingAction::DeletePlaylist { playlist_id } => {
            spawn_delete_playlist(state.clone(), ctx.clone(), playlist_id.clone());
            true
        }
        PendingAction::Subscribe { channel_id } => {
            spawn_subscribe(state.clone(), ctx.clone(), channel_id.clone());
            true
        }
        PendingAction::Unsubscribe { subscription_id } => {
            spawn_unsubscribe(state.clone(), ctx.clone(), subscription_id.clone());
            true
        }
        PendingAction::RateVideo { video_id, rating } => {
            spawn_rate_video(state.clone(), ctx.clone(), video_id.clone(), rating.clone());
            true
        }
        PendingAction::AddToPlaylist {
            playlist_id,
            playlist_title,
            video_id,
        } => {
            spawn_add_to_playlist(
                state.clone(),
                ctx.clone(),
                playlist_id.clone(),
                playlist_title.clone(),
                video_id.clone(),
            );
            true
        }
        PendingAction::ExportSubscriptionsOpml => {
            let mut s_lock = state.lock().unwrap();
            if let Some(Ok(ref subs)) = s_lock.subscriptions {
                let imports: Vec<youtube_client_lib::SubscriptionImport> = subs
                    .iter()
                    .map(youtube_client_lib::SubscriptionImport::from)
                    .collect();
                let opml = youtube_client_lib::export_subscriptions_to_opml(&imports);
                let export_path = s_lock.downloads_dir.join("youtube_subscriptions.opml");
                if let Err(e) = std::fs::create_dir_all(&s_lock.downloads_dir) {
                    s_lock.set_toast(format!("Failed to create directory: {e}"), true);
                } else {
                    match std::fs::write(&export_path, opml) {
                        Ok(_) => {
                            let count = imports.len();
                            s_lock.set_toast(
                                format!(
                                    "Exported {count} subscriptions to {}",
                                    export_path.display()
                                ),
                                false,
                            );
                        }
                        Err(e) => {
                            s_lock.set_toast(format!("Failed to write OPML file: {e}"), true);
                        }
                    }
                }
            } else {
                s_lock.set_toast("No subscriptions loaded yet to export.", true);
            }
            true
        }
        PendingAction::ImportSubscriptionsFile { path } => {
            let mut s_lock = state.lock().unwrap();
            let registry = youtube_client_lib::SubscriptionFormatRegistry::with_defaults();
            match registry.import_file(path) {
                Ok(imported) => {
                    let count = imported.len();
                    if count == 0 {
                        s_lock.set_toast("No channels found in file.", true);
                    } else {
                        let mut current = match s_lock.subscriptions.take() {
                            Some(Ok(subs)) => subs,
                            _ => Vec::new(),
                        };
                        for imp in imported {
                            if !current.iter().any(|s| s.channel_id == imp.channel_id) {
                                current.push(youtube_client_lib::Subscription {
                                    id: format!("import_{}", imp.channel_id),
                                    title: imp.channel_title,
                                    description: String::new(),
                                    channel_id: imp.channel_id,
                                    thumbnail_url: String::new(),
                                });
                            }
                        }
                        s_lock.subscriptions = Some(Ok(current));
                        s_lock.set_toast(
                            format!(
                                "Imported {count} subscriptions from {}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ),
                            false,
                        );
                    }
                }
                Err(e) => {
                    s_lock.set_toast(format!("Failed to import subscriptions: {e}"), true);
                }
            }
            true
        }
        _ => false,
    }
}
