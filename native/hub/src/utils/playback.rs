use std::path::Path;
use std::sync::Arc;

use rinf::{DartSignal, RustSignal};
use tokio::spawn;

use crate::signals::playback::{
    AudioProbe, GetHistoryRequest, GetHistoryResponse, PlayTrackRequest, PlayTrackResponse,
    PlaybackRecord, PreviewTrackRequest, PreviewTrackResponse, ProbeStreamRequest,
    ProbeStreamResponse, ReportPlaybackRequest, ReportPlaybackResponse, UpdateNowPlayingRequest,
    UpdateNowPlayingResponse,
};
use crate::utils::logger;

use tawai_core::app_context::AppContext;
use tawai_core::audio::probe::{probe_file, probe_url};
use tawai_core::db::{database::DatabasePool, history, library};
use tawai_core::dclient::iprev;
use tawai_core::discovery::listenbrainz;
use tawai_core::utils::playback::resolve_playable_track;

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

async fn probe_local_file(path: String) -> Result<AudioProbe, String> {
    tokio::task::spawn_blocking(move || probe_file(Path::new(&path)))
        .await
        .map_err(|error| format!("probe task failed: {error}"))?
        .map(AudioProbe::from)
        .map_err(|error| error.to_string())
}

async fn probe_remote(url: &str, headers: &[(String, String)]) -> Result<AudioProbe, String> {
    probe_url(url, headers)
        .await
        .map(AudioProbe::from)
        .map_err(|error| error.to_string())
}

async fn probe_track(
    context: &Arc<AppContext>,
    id: String,
    msg: ProbeStreamRequest,
) -> ProbeStreamResponse {
    if let Some(track) = &msg.track {
        if track.source == "external" || track.source_type == "external" {
            let result = probe_local_file(track.file_path.clone()).await;
            return match result {
                Ok(info) => ProbeStreamResponse {
                    id,
                    mode: Some("external".to_string()),
                    info: Some(info),
                    error: None,
                },
                Err(error) => ProbeStreamResponse {
                    id,
                    mode: Some("external".to_string()),
                    info: None,
                    error: Some(error.to_string()),
                },
            };
        }

        if track.source == "preview" || track.source_type == "preview" {
            return probe_preview(context, id, track).await;
        }
    }

    let db = context.db().await;
    let cfg = context.cfg().await;
    let master_key = context.master_key.read().await.clone();

    let result = resolve_playable_track(
        db.pool(),
        context.client(),
        msg.track_id.as_deref(),
        msg.track.as_ref().map(|track| track.title.as_str()),
        msg.track
            .as_ref()
            .map(|track| track.artists_string.as_str()),
        msg.track.as_ref().map(|track| track.album_title.as_str()),
        msg.track
            .as_ref()
            .and_then(|track| track.mbid_recording.as_deref()),
        Some(&cfg),
        &master_key,
        Arc::downgrade(context),
    )
    .await;

    if let Some(error) = result.error {
        return ProbeStreamResponse {
            id,
            mode: None,
            info: None,
            error: Some(error),
        };
    }
    if result.file_path.is_empty() {
        return ProbeStreamResponse {
            id,
            mode: None,
            info: None,
            error: Some("No audio source found".to_string()),
        };
    }

    let (mode, probe) = if is_url(&result.file_path) {
        let headers = result.headers.clone().unwrap_or_default();
        (
            "direct".to_string(),
            probe_remote(&result.file_path, &headers).await,
        )
    } else {
        (
            "local".to_string(),
            probe_local_file(result.file_path.clone()).await,
        )
    };

    match probe {
        Ok(info) => ProbeStreamResponse {
            id,
            mode: Some(mode),
            info: Some(info),
            error: None,
        },
        Err(error) => {
            logger::error(&format!("stream probe failed: {error}"));
            ProbeStreamResponse {
                id,
                mode: Some(mode),
                info: None,
                error: Some(error.to_string()),
            }
        }
    }
}

async fn probe_preview(
    context: &Arc<AppContext>,
    id: String,
    track: &crate::signals::library::TrackInfo,
) -> ProbeStreamResponse {
    match iprev::fetch_preview(context.client(), &track.artists_string, &track.title).await {
        Ok(Some(url)) => match probe_remote(&url, &[]).await {
            Ok(mut info) => {
                info.transport = Some("preview".to_string());
                ProbeStreamResponse {
                    id,
                    mode: Some("preview".to_string()),
                    info: Some(info),
                    error: None,
                }
            }
            Err(error) => ProbeStreamResponse {
                id,
                mode: Some("preview".to_string()),
                info: None,
                error: Some(error.to_string()),
            },
        },
        Ok(None) => ProbeStreamResponse {
            id,
            mode: Some("preview".to_string()),
            info: None,
            error: Some("No preview available".to_string()),
        },
        Err(error) => ProbeStreamResponse {
            id,
            mode: Some("preview".to_string()),
            info: None,
            error: Some(error.to_string()),
        },
    }
}

pub async fn handle_probe_stream(context: Arc<AppContext>) {
    let receiver = ProbeStreamRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        // Probe concurrently: a slow URL probe for a skipped track must not
        // delay the probe for the track actually playing.
        let context = context.clone();
        spawn(async move {
            let id = msg.id.clone();
            let response = probe_track(&context, id, msg).await;
            response.send_signal_to_dart();
        });
    }
}

pub async fn handle_play_track(context: Arc<AppContext>) {
    let receiver = PlayTrackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let cfg = context.cfg().await;
        let mk = context.master_key.read().await.clone();

        let result = resolve_playable_track(
            db.pool(),
            context.client(),
            msg.track_id.as_deref(),
            msg.track.as_ref().map(|t| t.title.as_str()),
            msg.track.as_ref().map(|t| t.artists_string.as_str()),
            msg.track.as_ref().map(|t| t.album_title.as_str()),
            msg.track.as_ref().and_then(|t| t.mbid_recording.as_deref()),
            Some(&cfg),
            &mk,
            Arc::downgrade(&context),
        )
        .await;

        PlayTrackResponse {
            id: msg.id,
            file_path: result.file_path,
            error: result.error,
            headers: result.headers,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_preview_track(context: Arc<AppContext>) {
    let receiver = PreviewTrackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        match iprev::fetch_preview(
            context.client(),
            &msg.track.artists_string,
            &msg.track.title,
        )
        .await
        {
            Ok(Some(url)) => {
                PreviewTrackResponse {
                    id: msg.id,
                    url: Some(url),
                    source: Some("itunes".to_string()),
                    error: None,
                }
                .send_signal_to_dart();
            }
            Ok(None) => {
                PreviewTrackResponse {
                    id: msg.id,
                    url: None,
                    source: None,
                    error: Some("No preview available".to_string()),
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("preview failed: {}", e));
                PreviewTrackResponse {
                    id: msg.id,
                    url: None,
                    source: None,
                    error: Some(e.to_string()),
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_report_playback(context: Arc<AppContext>) {
    let receiver = ReportPlaybackRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        let history_id =
            history::record_playback(db.pool(), &msg.user_id, &msg.track_id, &msg.source).await;

        match history_id {
            Ok(hid) => {
                if msg.source != "jellyfin" {
                    let mk = context.master_key.read().await.clone();
                    let token = history::get_listenbrainz_token(db.pool(), &msg.user_id, &mk).await;
                    if let Some(lb_token) = token {
                        if let Ok(Some(track)) =
                            library::lookup_track(db.pool(), &msg.track_id).await
                        {
                            let client = context.client().clone();
                            let pool_clone = match db.pool() {
                                DatabasePool::Sqlite(p) => DatabasePool::Sqlite(p.clone()),
                                DatabasePool::Postgres(p) => DatabasePool::Postgres(p.clone()),
                            };
                            spawn(async move {
                                match tawai_core::discovery::listenbrainz::scrobble(
                                    &client, &lb_token, &track,
                                )
                                .await
                                {
                                    Ok(()) => {
                                        let _ = history::mark_scrobbled(&pool_clone, &hid).await;
                                    }
                                    Err(e) => {
                                        logger::error(&format!("scrobble failed: {}", e));
                                    }
                                }
                            });
                        }
                    }
                }

                ReportPlaybackResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("record playback failed: {}", e));
                ReportPlaybackResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_update_now_playing(context: Arc<AppContext>) {
    let receiver = UpdateNowPlayingRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;

        match library::lookup_track(db.pool(), &msg.track_id).await {
            Ok(Some(track)) => {
                let mk = context.master_key.read().await.clone();
                let token = history::get_listenbrainz_token(db.pool(), &msg.user_id, &mk).await;

                if let Some(lb_token) = token {
                    let client = context.client().clone();
                    spawn(async move {
                        if let Err(e) =
                            listenbrainz::update_now_playing(&client, &lb_token, &track).await
                        {
                            logger::error(&format!("update now playing failed: {}", e));
                        }
                    });
                }

                UpdateNowPlayingResponse {
                    id: msg.id,
                    success: true,
                }
                .send_signal_to_dart();
            }
            Ok(None) => {
                UpdateNowPlayingResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("update now playing failed: {}", e));
                UpdateNowPlayingResponse {
                    id: msg.id,
                    success: false,
                }
                .send_signal_to_dart();
            }
        }
    }
}

pub async fn handle_get_history(context: Arc<AppContext>) {
    let receiver = GetHistoryRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let limit = msg.limit.unwrap_or(50);

        match history::get_recent_history(db.pool(), &msg.user_id, limit).await {
            Ok(recs) => {
                let entries: Vec<PlaybackRecord> = recs
                    .into_iter()
                    .map(|r| PlaybackRecord {
                        id: r.id,
                        track_id: r.track_id,
                        track_title: r.track_title,
                        album_title: r.album_title,
                        artist_name: r.artist_name,
                        played_at: r.played_at,
                        source: r.source,
                        scrobbled: r.scrobbled,
                        duration_secs: r.duration_secs,
                    })
                    .collect();
                GetHistoryResponse {
                    id: msg.id,
                    records: entries,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("get history failed: {}", e));
                GetHistoryResponse {
                    id: msg.id,
                    records: vec![],
                }
                .send_signal_to_dart();
            }
        }
    }
}
