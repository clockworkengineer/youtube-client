use eframe::egui;
use std::sync::{Arc, Mutex};
use youtube_client_lib::utils::{DownloadStatus, get_download_path};
use youtube_client_lib::{
    ChannelDetails, Comment, Page, Video, VideoDetails, YoutubeBackend, YoutubeClient,
};

use crate::types::{AppState, View, lock_state};

pub async fn get_client_async(state: &Arc<Mutex<AppState>>) -> Result<YoutubeBackend, String> {
    {
        let s = lock_state(state);
        if let Some(mock) = &s.mock_backend {
            return Ok(YoutubeBackend::mock(mock.clone()));
        }
        if let Some(backend) = &s.cached_backend {
            return Ok(backend.clone());
        }
    }

    let (client_id, client_secret) =
        youtube_client_lib::resolve_credentials(None, None, std::path::Path::new("config.json"))
            .map_err(|e| e.to_string())?;

    let token_cache_path = youtube_client_lib::resolve_token_cache_path();
    if !token_cache_path.exists() {
        return Err("Not signed in yet. Please sign in with your Google account.".to_string());
    }

    let has_scope = youtube_client_lib::check_token_cache_scopes(
        &token_cache_path,
        &[
            "https://www.googleapis.com/auth/youtube.readonly",
            "https://www.googleapis.com/auth/youtube",
            "https://www.googleapis.com/auth/youtube.force-ssl",
        ],
    );

    if !has_scope {
        return Err(
            "Authorization expired or missing permissions.\n\nPlease sign in again with Google."
                .to_string(),
        );
    }

    let client = YoutubeClient::builder()
        .with_credentials(&client_id, &client_secret)
        .with_token_cache(&token_cache_path)
        .with_scopes(youtube_client_lib::YOUTUBE_SCOPES)
        .with_default_cache()
        .build()
        .await
        .map_err(|e| format!("Authentication failed: {e}"))?;

    let backend = YoutubeBackend::live(client);
    {
        let mut s = lock_state(state);
        s.cached_backend = Some(backend.clone());
    }
    Ok(backend)
}

pub fn spawn_client_action<F, Fut, T>(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    action_name: &'static str,
    f: F,
    on_complete: impl FnOnce(Result<T, String>, &mut AppState, &egui::Context) + Send + 'static,
) where
    F: FnOnce(YoutubeBackend) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
    T: Send + 'static,
{
    let state_clone = state.clone();
    tokio::spawn(async move {
        let res = async {
            let client = get_client_async(&state_clone).await?;
            f(client).await
        }
        .await;

        let mut s = lock_state(&state_clone);
        if let Err(e) = &res {
            youtube_client_lib::utils::append_to_log(
                &s.log_file,
                "ERROR",
                &format!("Error during {action_name}: {e}"),
            );
        } else {
            let cost = match action_name {
                "search_videos" => youtube_client_lib::costs::SEARCH,
                "rate_video"
                | "post_comment"
                | "add_to_playlist"
                | "remove_from_playlist"
                | "create_playlist"
                | "delete_playlist"
                | "unsubscribe"
                | "subscribe" => youtube_client_lib::costs::WRITE,
                "fetch_new_videos" => 6,
                _ => youtube_client_lib::costs::READ,
            };
            s.quota_tracker.record_cost(cost);
            let _ = s.quota_tracker.save_to_file(&s.quota_path);
        }
        on_complete(res, &mut s, &ctx);
    });
}

pub fn spawn_fetch_subscriptions(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
    spawn_client_action(
        state,
        ctx,
        "fetch_subscriptions",
        |client| async move {
            client
                .list_subscriptions(50)
                .await
                .map_err(|e| format!("Failed to fetch subscriptions: {e}"))
        },
        |res, s, ctx| {
            match res {
                Ok(subs) => {
                    s.subscriptions = Some(Ok(Arc::new(subs)));
                }
                Err(e) => {
                    s.subscriptions = Some(Err(e));
                    s.current_view = View::Login;
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_fetch_new_videos(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
    spawn_client_action(
        state,
        ctx,
        "fetch_new_videos",
        |client| async move {
            let subs = client
                .list_subscriptions(10)
                .await
                .map_err(|e| format!("Failed to fetch subscriptions for new videos feed: {e}"))?;

            let client_arc = Arc::new(client);
            let mut handles = Vec::new();
            for sub in subs.into_iter().take(5) {
                let client_ref = client_arc.clone();
                handles.push(tokio::spawn(async move {
                    client_ref.list_videos(&sub.channel_id, 5).await.ok()
                }));
            }

            let mut all_videos = Vec::new();
            for handle in handles {
                if let Ok(Some(vids)) = handle.await {
                    all_videos.extend(vids);
                }
            }
            all_videos.sort_by(|a, b| b.published_at.cmp(&a.published_at));
            Ok(all_videos)
        },
        |res, s, ctx| {
            match res {
                Ok(mut vids) => {
                    vids.retain(|v| !s.cleared_video_ids.contains(&v.id));
                    s.new_videos = Some(Ok(Arc::new(vids)));
                }
                Err(e) => {
                    s.new_videos = Some(Err(e));
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_login_and_auth(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    id: String,
    secret: String,
) {
    {
        let mut s = lock_state(&state);
        s.logging_in = true;
        s.login_error = None;
        s.cached_backend = None;
    }
    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let res = async {
            let mut config = youtube_client_lib::load_config();
            config.client_id = Some(id.clone());
            config.client_secret = Some(secret.clone());
            youtube_client_lib::save_config(&config).map_err(|e| e.to_string())?;

            let token_cache_path = youtube_client_lib::resolve_token_cache_path();
            let client = YoutubeClient::new_oauth_with_scopes(
                &id,
                &secret,
                &token_cache_path,
                youtube_client_lib::YOUTUBE_SCOPES,
            )
            .await
            .map_err(|e| format!("OAuth initialization failed: {e}"))?;
            client
                .test_connection()
                .await
                .map_err(|e| format!("YouTube connection failed: {e}"))?;
            Ok(())
        }
        .await;

        let mut s = lock_state(&state_clone);
        if !s.logging_in {
            // Cancelled by user
            return;
        }
        s.logging_in = false;
        match res {
            Ok(_) => {
                s.cached_backend = None;
                s.current_view = View::Subscriptions;
                s.subscriptions = None;
                s.set_toast("Signed in successfully!", false);
                drop(s);
                spawn_fetch_subscriptions(state_clone, ctx_clone);
            }
            Err(e) => {
                s.login_error = Some(e);
                ctx_clone.request_repaint();
            }
        }
    });
}

pub fn spawn_default_login(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
    {
        let mut s = lock_state(&state);
        s.logging_in = true;
        s.login_error = None;
        s.cached_backend = None;
    }
    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let res = async {
            let (client_id, client_secret) = youtube_client_lib::resolve_credentials(
                None,
                None,
                std::path::Path::new("config.json"),
            )
            .map_err(|e| format!("Could not resolve credentials: {e}"))?;

            let token_cache_path = youtube_client_lib::resolve_token_cache_path();
            let client = YoutubeClient::new_oauth_with_scopes(
                &client_id,
                &client_secret,
                &token_cache_path,
                youtube_client_lib::YOUTUBE_SCOPES,
            )
            .await
            .map_err(|e| format!("OAuth authentication failed: {e}"))?;

            client
                .test_connection()
                .await
                .map_err(|e| format!("YouTube connection failed: {e}"))?;
            Ok(())
        }
        .await;

        let mut s = lock_state(&state_clone);
        if !s.logging_in {
            // Cancelled by user
            return;
        }
        s.logging_in = false;
        match res {
            Ok(_) => {
                s.cached_backend = None;
                s.current_view = View::Subscriptions;
                s.subscriptions = None;
                s.set_toast("Signed in successfully!", false);
                drop(s);
                spawn_fetch_subscriptions(state_clone, ctx_clone);
            }
            Err(e) => {
                s.login_error = Some(e);
                ctx_clone.request_repaint();
            }
        }
    });
}

pub fn spawn_download(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    video: Video,
    is_audio: bool,
    resolution: Option<youtube_client_lib::download::VideoResolution>,
    audio_bitrate: Option<youtube_client_lib::download::AudioBitrate>,
) {
    let video_id = video.id.clone();
    let video_title = video.title.clone();
    {
        let mut s = lock_state(&state);
        s.downloads.insert(
            video_id.clone(),
            DownloadStatus::Downloading {
                progress: "Starting...".to_string(),
            },
        );
        let kind = if is_audio {
            "audio (MP3)"
        } else {
            "video (MP4)"
        };
        s.set_toast(
            format!("Started downloading {kind}: {}", video.title),
            false,
        );
    }
    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let res: Result<std::path::PathBuf, String> = async {
            let (downloads_base, log_file) = {
                let s = lock_state(&state_clone);
                (s.downloads_dir.clone(), s.log_file.clone())
            };

            let output_path = get_download_path(
                &downloads_base,
                &video.channel_title,
                &video.title,
                &video.id,
                is_audio,
            );

            if let Some(parent) = output_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }

            let client = get_client_async(&state_clone)
                .await
                .map_err(|e| e.to_string())?;

            let format = if is_audio {
                youtube_client_lib::download::DownloadFormat::Mp3
            } else {
                youtube_client_lib::download::DownloadFormat::Mp4
            };
            let mut options =
                youtube_client_lib::download::DownloadOptions::new(format).with_log_file(log_file);

            if let Some(res) = resolution {
                options = options.with_resolution(res);
            } else {
                let config = youtube_client_lib::load_config();
                if let Some(ref r) = config.preferred_resolution {
                    options = options.with_resolution(
                        youtube_client_lib::download::VideoResolution::from_str_loose(r),
                    );
                }
            }

            if let Some(ab) = audio_bitrate {
                options = options.with_audio_bitrate(ab);
            } else {
                let config = youtube_client_lib::load_config();
                if let Some(ref a) = config.preferred_audio_bitrate {
                    options = options.with_audio_bitrate(
                        youtube_client_lib::download::AudioBitrate::from_str_loose(a),
                    );
                }
            }

            let state_inner = state_clone.clone();
            let ctx_inner = ctx_clone.clone();
            let vid_id = video_id.clone();
            client
                .download_video_with_options(&video_id, &output_path, &options, move |prog| {
                    let mut s = lock_state(&state_inner);
                    s.downloads.insert(
                        vid_id.clone(),
                        DownloadStatus::Downloading {
                            progress: prog.to_string(),
                        },
                    );
                    ctx_inner.request_repaint();
                })
                .await
                .map_err(|e| e.to_string())?;

            Ok(output_path)
        }
        .await;

        let mut s = lock_state(&state_clone);
        match res {
            Ok(path) => {
                s.downloads.insert(video_id, DownloadStatus::Finished(path));
                s.set_toast(format!("Download complete: {video_title}"), false);
            }
            Err(e) => {
                s.downloads
                    .insert(video_id, DownloadStatus::Failed(e.clone()));
                s.set_toast(format!("Download failed: {e}"), true);
            }
        }
        ctx_clone.request_repaint();
    });
}

pub fn fetch_videos(
    ctx: egui::Context,
    state: Arc<Mutex<AppState>>,
    channel_id: String,
    _channel_title: String,
) {
    let channel_id_clone = channel_id.clone();
    spawn_client_action(
        state,
        ctx,
        "fetch_videos",
        move |client| async move {
            let details_res = client
                .get_channel_details(&channel_id)
                .await
                .map_err(|e| format!("Failed to fetch channel stats: {e}"));

            let videos_res = match client.list_videos(&channel_id, 20).await {
                Ok(vids) => Ok(vids),
                Err(err) => {
                    let http = youtube_client_lib::utils::default_http_client();
                    if let Ok(rss_vids) =
                        youtube_client_lib::fetch_channel_videos_via_rss(&http, &channel_id).await
                    {
                        if !rss_vids.is_empty() {
                            Ok(rss_vids)
                        } else {
                            Err(format!("Failed to fetch videos: {err}"))
                        }
                    } else {
                        Err(format!("Failed to fetch videos: {err}"))
                    }
                }
            };
            Ok((details_res, videos_res))
        },
        move |res: Result<(Result<ChannelDetails, String>, Result<Vec<Video>, String>), String>,
              s,
              ctx| {
            if let View::ChannelVideos {
                channel_id: current_id,
                channel_title: current_title,
                channel_description: current_desc,
                ..
            } = &s.current_view
            {
                if current_id == &channel_id_clone {
                    let (channel_details, videos) = match res {
                        Ok((d, v)) => (Some(d.map(Arc::new)), Some(v.map(Arc::new))),
                        Err(e) => (Some(Err(e.clone())), Some(Err(e))),
                    };
                    s.current_view = View::ChannelVideos {
                        channel_id: channel_id_clone,
                        channel_title: current_title.clone(),
                        channel_description: current_desc.clone(),
                        channel_details,
                        videos,
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn fetch_search_results(
    ctx: egui::Context,
    state: Arc<Mutex<AppState>>,
    query: String,
    filter: Option<youtube_client_lib::SearchFilter>,
) {
    let query_clone = query.clone();
    spawn_client_action(
        state,
        ctx,
        "search_videos",
        move |client| async move {
            client
                .search_videos_with_filter(&query, 20, filter.as_ref())
                .await
                .map_err(|e| format!("Failed to search videos: {e}"))
        },
        move |res, s, ctx| {
            if let View::SearchResults {
                query: current_q,
                videos: _,
            } = &s.current_view
            {
                if current_q == &query_clone {
                    s.current_view = View::SearchResults {
                        query: query_clone,
                        videos: Some(res.map(Arc::new)),
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_fetch_playlists(state: Arc<Mutex<AppState>>, ctx: egui::Context) {
    spawn_client_action(
        state,
        ctx,
        "fetch_playlists",
        |client| async move {
            client
                .list_playlists(50)
                .await
                .map_err(|e| format!("Failed to fetch playlists: {e}"))
        },
        |res, s, ctx| {
            s.playlists = Some(res.clone().map(Arc::new));
            if let View::Playlists { playlists: _ } = &s.current_view {
                s.current_view = View::Playlists {
                    playlists: Some(res.map(Arc::new)),
                };
            }
            ctx.request_repaint();
        },
    );
}

pub fn fetch_playlist_videos(
    ctx: egui::Context,
    state: Arc<Mutex<AppState>>,
    playlist_id: String,
    _playlist_title: String,
) {
    let playlist_id_clone = playlist_id.clone();
    spawn_client_action(
        state,
        ctx,
        "fetch_playlist_videos",
        move |client| async move {
            client
                .list_playlist_videos(&playlist_id, 50)
                .await
                .map_err(|e| format!("Failed to fetch playlist videos: {e}"))
        },
        move |res, s, ctx| {
            if let View::PlaylistVideos {
                playlist_id: current_id,
                playlist_title: current_title,
                videos: _,
            } = &s.current_view
            {
                if current_id == &playlist_id_clone {
                    s.current_view = View::PlaylistVideos {
                        playlist_id: playlist_id_clone,
                        playlist_title: current_title.clone(),
                        videos: Some(res.map(Arc::new)),
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_fetch_comments(state: Arc<Mutex<AppState>>, ctx: egui::Context, video: Video) {
    let video_clone = video.clone();
    let vid_id = video.id.clone();

    // Also fetch SponsorBlock segments in parallel if SponsorBlock is enabled
    let config = youtube_client_lib::load_config();
    if config.sponsorblock_enabled.unwrap_or(true) {
        let sb_state = state.clone();
        let sb_ctx = ctx.clone();
        let sb_vid_id = vid_id.clone();
        let sb_cats = config.sponsorblock_categories.clone();
        tokio::spawn(async move {
            let http = youtube_client_lib::utils::default_http_client();
            let cat_refs: Option<Vec<&str>> = sb_cats
                .as_ref()
                .map(|v| v.iter().map(|s| s.as_str()).collect());
            if let Ok(segments) = youtube_client_lib::sponsorblock::fetch_skip_segments(
                &http,
                &sb_vid_id,
                cat_refs.as_deref(),
            )
            .await
            {
                let mut s = lock_state(&sb_state);
                s.sponsor_segments.insert(sb_vid_id, segments);
                sb_ctx.request_repaint();
            }
        });
    }

    spawn_client_action(
        state,
        ctx,
        "fetch_video_details_and_comments",
        move |client| async move {
            let details_res = client
                .fetch_video_details(&vid_id)
                .await
                .map_err(|e| e.to_string());
            let comments_res = client
                .fetch_comments_page(&vid_id, 20, None)
                .await
                .map_err(|e| e.to_string());
            Ok((details_res, comments_res))
        },
        move |res: Result<
            (Result<VideoDetails, String>, Result<Page<Comment>, String>),
            String,
        >,
              s,
              ctx| {
            if let View::VideoDetails {
                video: current_video,
                ..
            } = &s.current_view
            {
                if current_video.id == video_clone.id {
                    let (details, comments, next_token) = match res {
                        Ok((d, Ok(page))) => (
                            Some(d.map(Arc::new)),
                            Some(Ok(Arc::new(page.items))),
                            page.next_page_token,
                        ),
                        Ok((d, Err(c_err))) => (Some(d.map(Arc::new)), Some(Err(c_err)), None),
                        Err(e) => (Some(Err(e.clone())), Some(Err(e)), None),
                    };
                    s.current_view = View::VideoDetails {
                        video: video_clone,
                        details,
                        comments,
                        comments_next_page_token: next_token,
                        loading_more_comments: false,
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_fetch_more_comments(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    video_id: String,
    page_token: String,
) {
    {
        let mut s = lock_state(&state);
        if let View::VideoDetails {
            video: current_video,
            loading_more_comments,
            ..
        } = &mut s.current_view
        {
            if current_video.id == video_id {
                *loading_more_comments = true;
            }
        }
    }

    let vid_clone = video_id.clone();
    spawn_client_action(
        state,
        ctx,
        "fetch_more_comments",
        move |client| async move {
            client
                .fetch_comments_page(&video_id, 20, Some(&page_token))
                .await
                .map_err(|e| format!("Failed to load more comments: {e}"))
        },
        move |res, s, ctx| {
            if let View::VideoDetails {
                video: current_video,
                comments: Some(Ok(list)),
                comments_next_page_token,
                loading_more_comments,
                ..
            } = &mut s.current_view
            {
                if current_video.id == vid_clone {
                    *loading_more_comments = false;
                    match res {
                        Ok(page) => {
                            Arc::make_mut(list).extend(page.items);
                            *comments_next_page_token = page.next_page_token;
                        }
                        Err(err) => {
                            s.set_toast(err, true);
                        }
                    }
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_post_comment(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    video_id: String,
    text: String,
) {
    let video_id_clone = video_id.clone();
    spawn_client_action(
        state,
        ctx,
        "post_comment",
        move |client| async move {
            client
                .post_comment(&video_id, &text)
                .await
                .map_err(|e| format!("Failed to post comment: {e}"))
        },
        move |res, s, ctx| {
            match res {
                Ok(new_comment) => {
                    if let View::VideoDetails {
                        video: current_video,
                        comments: Some(Ok(list)),
                        ..
                    } = &mut s.current_view
                    {
                        if current_video.id == video_id_clone {
                            Arc::make_mut(list).insert(0, new_comment);
                        }
                    }
                    s.set_toast("Comment posted successfully!", false);
                }
                Err(err) => {
                    s.set_toast(err, true);
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_create_playlist(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    title: String,
    description: Option<String>,
) {
    let title_clone = title.clone();
    spawn_client_action(
        state.clone(),
        ctx.clone(),
        "create_playlist",
        move |client| async move {
            client
                .create_playlist(&title, description.as_deref())
                .await
                .map_err(|e| format!("Failed to create playlist: {e}"))
        },
        move |res, s, ctx| {
            match res {
                Ok(new_pl) => {
                    if let Some(Ok(list)) = &mut s.playlists {
                        Arc::make_mut(list).insert(0, new_pl.clone());
                    }
                    if let View::Playlists {
                        playlists: Some(Ok(list)),
                    } = &mut s.current_view
                    {
                        Arc::make_mut(list).insert(0, new_pl);
                    }
                    s.set_toast(format!("Playlist '{title_clone}' created!"), false);
                }
                Err(err) => {
                    s.set_toast(err, true);
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_delete_playlist(state: Arc<Mutex<AppState>>, ctx: egui::Context, playlist_id: String) {
    let pid_clone = playlist_id.clone();
    spawn_client_action(
        state,
        ctx,
        "delete_playlist",
        move |client| async move {
            client
                .delete_playlist(&playlist_id)
                .await
                .map_err(|e| format!("Failed to delete playlist: {e}"))
        },
        move |res, s, ctx| {
            match res {
                Ok(()) => {
                    if let Some(Ok(list)) = &mut s.playlists {
                        Arc::make_mut(list).retain(|p| p.id != pid_clone);
                    }
                    if let View::Playlists {
                        playlists: Some(Ok(list)),
                    } = &mut s.current_view
                    {
                        Arc::make_mut(list).retain(|p| p.id != pid_clone);
                    }
                    s.set_toast("Playlist deleted.", false);
                }
                Err(err) => {
                    s.set_toast(err, true);
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_subscribe(state: Arc<Mutex<AppState>>, ctx: egui::Context, channel_id: String) {
    let state_clone = state.clone();
    spawn_client_action(
        state,
        ctx,
        "subscribe",
        move |client| async move {
            client
                .subscribe_to_channel(&channel_id)
                .await
                .map_err(|e| format!("Failed to subscribe: {e}"))
        },
        move |res, s, ctx| match res {
            Ok(_) => {
                s.set_toast("Subscribed to channel!", false);
                spawn_fetch_subscriptions(state_clone, ctx.clone());
            }
            Err(err) => {
                s.set_toast(err, true);
                ctx.request_repaint();
            }
        },
    );
}

pub fn spawn_unsubscribe(state: Arc<Mutex<AppState>>, ctx: egui::Context, subscription_id: String) {
    let state_clone = state.clone();
    spawn_client_action(
        state,
        ctx,
        "unsubscribe",
        move |client| async move {
            client
                .unsubscribe_from_channel(&subscription_id)
                .await
                .map_err(|e| format!("Failed to unsubscribe: {e}"))
        },
        move |res, s, ctx| match res {
            Ok(()) => {
                s.set_toast("Unsubscribed from channel.", false);
                spawn_fetch_subscriptions(state_clone, ctx.clone());
            }
            Err(err) => {
                s.set_toast(err, true);
                ctx.request_repaint();
            }
        },
    );
}

pub fn spawn_rate_video(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    video_id: String,
    rating: String,
) {
    spawn_client_action(
        state,
        ctx,
        "rate_video",
        move |client| async move {
            client
                .rate_video(&video_id, &rating)
                .await
                .map_err(|e| format!("Failed to rate: {e}"))?;
            Ok((video_id, rating))
        },
        move |res, s, ctx| {
            match res {
                Ok((vid, rat)) => {
                    youtube_client_lib::utils::append_to_log(
                        &s.log_file,
                        "INFO",
                        &format!("Successfully rated video {vid} as {rat}"),
                    );
                    let msg = match rat.as_str() {
                        "like" => "Liked video 👍",
                        "dislike" => "Disliked video 👎",
                        _ => "Removed video rating",
                    };
                    s.set_toast(msg, false);
                }
                Err(err) => {
                    s.set_toast(err, true);
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_add_to_playlist(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    playlist_id: String,
    playlist_title: String,
    video_id: String,
) {
    spawn_client_action(
        state,
        ctx,
        "add_to_playlist",
        move |client| async move {
            client
                .add_to_playlist(&playlist_id, &video_id)
                .await
                .map_err(|e| format!("Failed to add to playlist: {e}"))?;
            Ok(format!("Added to '{playlist_title}'"))
        },
        move |res, s, ctx| {
            match &res {
                Ok(msg) => s.set_toast(msg.clone(), false),
                Err(err) => s.set_toast(err.clone(), true),
            }
            s.playlist_action_status = Some(res);
            ctx.request_repaint();
        },
    );
}

pub fn spawn_remove_from_playlist(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    playlist_id: String,
    playlist_title: String,
    playlist_item_id: String,
    video_id: String,
) {
    let pid_clone = playlist_id.clone();
    let pitem_clone = playlist_item_id.clone();
    let vid_clone = video_id.clone();
    spawn_client_action(
        state,
        ctx,
        "remove_from_playlist",
        move |client| async move {
            client
                .remove_from_playlist(&playlist_item_id)
                .await
                .map_err(|e| format!("Failed to remove from playlist: {e}"))?;
            Ok(format!("Removed video from '{playlist_title}'"))
        },
        move |res, s, ctx| {
            match res {
                Ok(msg) => {
                    if let View::PlaylistVideos {
                        playlist_id: current_id,
                        videos: Some(Ok(list)),
                        ..
                    } = &mut s.current_view
                    {
                        if current_id == &pid_clone {
                            Arc::make_mut(list).retain(|v| {
                                v.playlist_item_id.as_deref() != Some(pitem_clone.as_str())
                                    && v.id != vid_clone
                            });
                        }
                    }
                    s.set_toast(msg, false);
                }
                Err(err) => {
                    s.set_toast(err, true);
                }
            }
            ctx.request_repaint();
        },
    );
}

fn get_thumbnail_cache_path(url: &str) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cache_dir = youtube_client_lib::config::get_global_config_dir()
        .map(|d| d.join("thumbnails"))
        .unwrap_or_else(|| std::path::PathBuf::from("thumbnails"));

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash_val = hasher.finish();
    cache_dir.join(format!("{hash_val:016x}.bin"))
}

const MAX_THUMBNAIL_BYTES: usize = 10 * 1024 * 1024;

pub fn fetch_thumbnail(
    ctx: egui::Context,
    state: Arc<Mutex<AppState>>,
    http_client: reqwest::Client,
    channel_id: String,
    url: String,
) {
    tokio::spawn(async move {
        let success = async {
            let cache_path = get_thumbnail_cache_path(&url);
            let bytes = if cache_path.exists() {
                let meta = tokio::fs::metadata(&cache_path).await.ok()?;
                if meta.len() > MAX_THUMBNAIL_BYTES as u64 {
                    return None;
                }
                tokio::fs::read(&cache_path).await.ok()
            } else {
                let response = http_client.get(&url).send().await.ok()?;
                if response
                    .content_length()
                    .is_some_and(|len| len > MAX_THUMBNAIL_BYTES as u64)
                {
                    return None;
                }
                let fetched = response.bytes().await.ok()?;
                if fetched.len() > MAX_THUMBNAIL_BYTES {
                    return None;
                }
                if let Some(parent) = cache_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&cache_path, &fetched).await;
                Some(fetched.to_vec())
            }?;
            if bytes.len() > MAX_THUMBNAIL_BYTES {
                return None;
            }

            let (size, pixels) = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&bytes).ok()?;
                let size = [img.width() as usize, img.height() as usize];
                let rgba = img.to_rgba8();
                Some((size, rgba.into_raw()))
            })
            .await
            .ok()
            .flatten()?;

            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

            let texture = ctx.load_texture(
                format!("thumb_{channel_id}"),
                color_image,
                Default::default(),
            );

            let mut s = lock_state(&state);
            if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                t.texture = Some(texture);
                t.loading = false;
            }
            ctx.request_repaint();
            Some(())
        }
        .await;

        if success.is_none() {
            let mut s = lock_state(&state);
            if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                t.loading = false;
            }
        }
    });
}
