use std::sync::{Arc, Mutex};
use eframe::egui;
use youtube_client_lib::utils::{get_download_path, DownloadStatus};
use youtube_client_lib::{Comment, Video, VideoDetails, YoutubeClient};

use crate::types::{AppState, View};

pub async fn get_client_async() -> Result<YoutubeClient, String> {
    let (client_id, client_secret) = youtube_client_lib::resolve_credentials(
        None,
        None,
        std::path::Path::new("config.json"),
    )
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
        return Err("Authorization expired or missing permissions.\n\nPlease sign in again with Google.".to_string());
    }

    let client = YoutubeClient::new_oauth_with_scopes(
        &client_id,
        &client_secret,
        &token_cache_path,
        youtube_client_lib::YOUTUBE_SCOPES,
    )
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    Ok(client)
}

pub fn spawn_client_action<F, Fut, T>(
    state: Arc<Mutex<AppState>>,
    ctx: egui::Context,
    action_name: &'static str,
    f: F,
    on_complete: impl FnOnce(Result<T, String>, &mut AppState, &egui::Context) + Send + 'static,
) where
    F: FnOnce(YoutubeClient) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
    T: Send + 'static,
{
    let state_clone = state.clone();
    tokio::spawn(async move {
        let res = async {
            let client = get_client_async().await?;
            f(client).await
        }
        .await;

        if let Err(e) = &res {
            println!("Error during {}: {}", action_name, e);
        }
        let mut s = state_clone.lock().unwrap();
        on_complete(res, &mut *s, &ctx);
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
                .map_err(|e| format!("Failed to fetch subscriptions: {}", e))
        },
        |res, s, ctx| {
            match res {
                Ok(subs) => {
                    s.subscriptions = Some(Ok(subs));
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
                .map_err(|e| format!("Failed to fetch subscriptions for new videos feed: {}", e))?;

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
                    s.new_videos = Some(Ok(vids));
                }
                Err(e) => {
                    s.new_videos = Some(Err(e));
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_login_and_auth(state: Arc<Mutex<AppState>>, ctx: egui::Context, id: String, secret: String) {
    {
        let mut s = state.lock().unwrap();
        s.logging_in = true;
        s.login_error = None;
    }
    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let res = async {
            #[derive(serde::Serialize)]
            struct ConfigSave {
                client_id: String,
                client_secret: String,
            }
            let config_data = ConfigSave {
                client_id: id.clone(),
                client_secret: secret.clone(),
            };
            let content = serde_json::to_string_pretty(&config_data).map_err(|e| e.to_string())?;
            std::fs::write("private_config.json", content).map_err(|e| e.to_string())?;

            let token_cache_path = youtube_client_lib::resolve_token_cache_path();
            let client = YoutubeClient::new_oauth_with_scopes(
                &id,
                &secret,
                &token_cache_path,
                youtube_client_lib::YOUTUBE_SCOPES,
            )
            .await
            .map_err(|e| format!("OAuth initialization failed: {}", e))?;
            client
                .test_connection()
                .await
                .map_err(|e| format!("YouTube connection failed: {}", e))?;
            Ok(())
        }
        .await;

        let mut s = state_clone.lock().unwrap();
        s.logging_in = false;
        match res {
            Ok(_) => {
                s.current_view = View::Subscriptions;
                s.subscriptions = None;
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
        let mut s = state.lock().unwrap();
        s.logging_in = true;
        s.login_error = None;
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
            .map_err(|e| format!("Could not resolve credentials: {}", e))?;

            let token_cache_path = youtube_client_lib::resolve_token_cache_path();
            let client = YoutubeClient::new_oauth_with_scopes(
                &client_id,
                &client_secret,
                &token_cache_path,
                youtube_client_lib::YOUTUBE_SCOPES,
            )
            .await
            .map_err(|e| format!("OAuth authentication failed: {}", e))?;

            client
                .test_connection()
                .await
                .map_err(|e| format!("YouTube connection failed: {}", e))?;
            Ok(())
        }
        .await;

        let mut s = state_clone.lock().unwrap();
        s.logging_in = false;
        match res {
            Ok(_) => {
                s.current_view = View::Subscriptions;
                s.subscriptions = None;
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

pub fn spawn_download(state: Arc<Mutex<AppState>>, ctx: egui::Context, video: Video, is_audio: bool) {
    let video_id = video.id.clone();
    {
        let mut s = state.lock().unwrap();
        s.downloads.insert(
            video_id.clone(),
            DownloadStatus::Downloading {
                progress: "Starting...".to_string(),
            },
        );
    }
    let state_clone = state.clone();
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let res = async {
            let (downloads_base, log_file) = {
                let s = state_clone.lock().unwrap();
                (s.downloads_dir.clone(), s.log_file.clone())
            };

            let output_path =
                get_download_path(&downloads_base, &video.channel_title, &video.title, &video.id, is_audio);

            if let Some(parent) = output_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }

            let client = get_client_async().await.map_err(|e| e.to_string())?;

            let format = if is_audio {
                youtube_client_lib::download::DownloadFormat::Mp3
            } else {
                youtube_client_lib::download::DownloadFormat::Mp4
            };
            let options = youtube_client_lib::download::DownloadOptions::new(format)
                .with_log_file(log_file);

            let state_inner = state_clone.clone();
            let ctx_inner = ctx_clone.clone();
            let vid_id = video_id.clone();
            client
                .download_video_with_options(&video_id, &output_path, &options, move |prog| {
                    if let Ok(mut s) = state_inner.lock() {
                        s.downloads.insert(
                            vid_id.clone(),
                            DownloadStatus::Downloading {
                                progress: prog.to_string(),
                            },
                        );
                    }
                    ctx_inner.request_repaint();
                })
                .await
                .map_err(|e| e.to_string())?;

            Ok(output_path)
        }
        .await;

        let mut s = state_clone.lock().unwrap();
        match res {
            Ok(path) => {
                s.downloads.insert(video_id, DownloadStatus::Finished(path));
            }
            Err(e) => {
                s.downloads.insert(video_id, DownloadStatus::Failed(e));
            }
        }
        ctx_clone.request_repaint();
    });
}

pub fn fetch_videos(ctx: egui::Context, state: Arc<Mutex<AppState>>, channel_id: String, _channel_title: String) {
    let channel_id_clone = channel_id.clone();
    spawn_client_action(
        state,
        ctx,
        "fetch_videos",
        move |client| async move {
            client
                .list_videos(&channel_id, 20)
                .await
                .map_err(|e| format!("Failed to fetch videos: {}", e))
        },
        move |res, s, ctx| {
            if let View::ChannelVideos {
                channel_id: current_id,
                channel_title: current_title,
                channel_description: current_desc,
                videos: _,
            } = &s.current_view
            {
                if current_id == &channel_id_clone {
                    s.current_view = View::ChannelVideos {
                        channel_id: channel_id_clone,
                        channel_title: current_title.clone(),
                        channel_description: current_desc.clone(),
                        videos: Some(res),
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn fetch_search_results(ctx: egui::Context, state: Arc<Mutex<AppState>>, query: String) {
    let query_clone = query.clone();
    spawn_client_action(
        state,
        ctx,
        "fetch_search_results",
        move |client| async move {
            client
                .search_videos(&query, 20)
                .await
                .map_err(|e| format!("Failed to search videos: {}", e))
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
                        videos: Some(res),
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
                .map_err(|e| format!("Failed to fetch playlists: {}", e))
        },
        |res, s, ctx| {
            s.playlists = Some(res.clone());
            if let View::Playlists { playlists: _ } = &s.current_view {
                s.current_view = View::Playlists { playlists: Some(res) };
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
                .map_err(|e| format!("Failed to fetch playlist videos: {}", e))
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
                        videos: Some(res),
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
    spawn_client_action(
        state,
        ctx,
        "fetch_video_details_and_comments",
        move |client| async move {
            let details_res = client.fetch_video_details(&vid_id).await.map_err(|e| e.to_string());
            let comments_res = client.fetch_comments(&vid_id).await.map_err(|e| e.to_string());
            Ok((details_res, comments_res))
        },
        move |res: Result<(Result<VideoDetails, String>, Result<Vec<Comment>, String>), String>, s, ctx| {
            if let View::VideoDetails {
                video: current_video,
                ..
            } = &s.current_view
            {
                if current_video.id == video_clone.id {
                    let (details, comments) = match res {
                        Ok((d, c)) => (Some(d), Some(c)),
                        Err(e) => (Some(Err(e.clone())), Some(Err(e))),
                    };
                    s.current_view = View::VideoDetails {
                        video: video_clone,
                        details,
                        comments,
                    };
                }
            }
            ctx.request_repaint();
        },
    );
}

pub fn spawn_post_comment(state: Arc<Mutex<AppState>>, ctx: egui::Context, video_id: String, text: String) {
    let video_id_clone = video_id.clone();
    spawn_client_action(
        state,
        ctx,
        "post_comment",
        move |client| async move {
            client
                .post_comment(&video_id, &text)
                .await
                .map_err(|e| format!("Failed to post comment: {}", e))
        },
        move |res, s, ctx| {
            if let View::VideoDetails {
                video: current_video,
                comments: Some(Ok(list)),
                ..
            } = &mut s.current_view
            {
                if current_video.id == video_id_clone {
                    if let Ok(new_comment) = res {
                        list.insert(0, new_comment);
                    }
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
    spawn_client_action(
        state.clone(),
        ctx.clone(),
        "create_playlist",
        move |client| async move {
            client
                .create_playlist(&title, description.as_deref())
                .await
                .map_err(|e| format!("Failed to create playlist: {}", e))
        },
        move |res, s, ctx| {
            if let Ok(new_pl) = res {
                if let Some(Ok(list)) = &mut s.playlists {
                    list.insert(0, new_pl);
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
                .map_err(|e| format!("Failed to delete playlist: {}", e))
        },
        move |res, s, ctx| {
            if res.is_ok() {
                if let Some(Ok(list)) = &mut s.playlists {
                    list.retain(|p| p.id != pid_clone);
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
                .map_err(|e| format!("Failed to subscribe: {}", e))
        },
        move |res, _, ctx| {
            if res.is_ok() {
                spawn_fetch_subscriptions(state_clone, ctx.clone());
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
                .map_err(|e| format!("Failed to unsubscribe: {}", e))
        },
        move |res, _, ctx| {
            if res.is_ok() {
                spawn_fetch_subscriptions(state_clone, ctx.clone());
            }
        },
    );
}

pub fn spawn_rate_video(state: Arc<Mutex<AppState>>, ctx: egui::Context, video_id: String, rating: String) {
    spawn_client_action(
        state,
        ctx,
        "rate_video",
        move |client| async move {
            client
                .rate_video(&video_id, &rating)
                .await
                .map_err(|e| format!("Failed to rate: {}", e))?;
            Ok((video_id, rating))
        },
        move |res, _, _| {
            if let Ok((vid, rat)) = res {
                println!("Successfully rated video {} as {}", vid, rat);
            }
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
                .map_err(|e| format!("Failed to add to playlist: {}", e))?;
            Ok(format!("Added to '{}'", playlist_title))
        },
        move |res, s, ctx| {
            s.playlist_action_status = Some(res);
            ctx.request_repaint();
        },
    );
}

pub fn fetch_thumbnail(
    ctx: egui::Context,
    state: Arc<Mutex<AppState>>,
    http_client: reqwest::Client,
    channel_id: String,
    url: String,
) {
    tokio::spawn(async move {
        let success = async {
            let response = http_client.get(&url).send().await.ok()?;
            let bytes = response.bytes().await.ok()?;
            let img = image::load_from_memory(&bytes).ok()?;
            let size = [img.width() as _, img.height() as _];
            let rgba = img.to_rgba8();
            let pixels = rgba.into_raw();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

            let texture = ctx.load_texture(
                format!("thumb_{}", channel_id),
                color_image,
                Default::default(),
            );

            let mut s = state.lock().unwrap();
            if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                t.texture = Some(texture);
                t.loading = false;
            }
            ctx.request_repaint();
            Some(())
        }
        .await;

        if success.is_none() {
            let mut s = state.lock().unwrap();
            if let Some(t) = s.thumbnails.get_mut(&channel_id) {
                t.loading = false;
            }
        }
    });
}
