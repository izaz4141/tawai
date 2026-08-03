use std::sync::Arc;

use rinf::{DartSignal, RustSignal};
use tawai_core::app_context::AppContext;
use tawai_core::db::history;
use tawai_core::discovery::listenbrainz;

use crate::signals::discovery::{
    DiscoveryRecording, GetLBRecommendationsRequest, GetLBRecommendationsResponse,
    ValidateLBTokenRequest, ValidateLBTokenResponse,
};
use crate::utils::logger;
use tawai_core::tools::duplicates;

pub async fn handle_get_lb_recommendations(context: Arc<AppContext>) {
    let receiver = GetLBRecommendationsRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;
        let db = context.db().await;
        let mk = context.master_key.read().await.clone();

        let token = history::get_listenbrainz_token(db.pool(), &msg.user_id, &mk).await;

        let token = match token {
            Some(t) => t,
            None => {
                GetLBRecommendationsResponse {
                    id: msg.id,
                    recommendations: vec![],
                    playlist_title: None,
                    playlist_id: None,
                    playlist_count: None,
                    error: Some("ListenBrainz token not configured".to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let validated = listenbrainz::validate_token(context.client(), &token).await;
        let user_name = match validated {
            Ok(v) if v.valid => v.user_name.unwrap_or_default(),
            _ => {
                GetLBRecommendationsResponse {
                    id: msg.id,
                    recommendations: vec![],
                    playlist_title: None,
                    playlist_id: None,
                    playlist_count: None,
                    error: Some("Invalid ListenBrainz token".to_string()),
                }
                .send_signal_to_dart();
                continue;
            }
        };

        let source = tawai_core::libsources::RecommendationSource::from_api_rec_type(&msg.rec_type);

        let (mut hub_recs, playlist_title, playlist_id, playlist_count, error) = match source {
            Some(src) => match src.api_type {
                tawai_core::libsources::ApiType::Recommendations => {
                    match listenbrainz::fetch_recommendations(
                        context.client(),
                        &token,
                        &user_name,
                        &msg.rec_type,
                        msg.count,
                        msg.offset,
                    )
                    .await
                    {
                        Ok(recs) => {
                            let core: Vec<tawai_core::signals::discovery::DiscoveryRecording> =
                                recs.into_iter().map(Into::into).collect();
                            let hub: Vec<DiscoveryRecording> =
                                core.into_iter().map(Into::into).collect();
                            (hub, None, None, None, None)
                        }
                        Err(e) => (vec![], None, None, None, Some(e.to_string())),
                    }
                }
                tawai_core::libsources::ApiType::CreatedFor => {
                    let filter = src.created_for_filter();
                    let idx = msg.index.unwrap_or(0);
                    match listenbrainz::fetch_createdfor(
                        context.client(),
                        &token,
                        &user_name,
                        filter,
                        idx,
                    )
                    .await
                    {
                        Ok(cr) => {
                            let core: Vec<tawai_core::signals::discovery::DiscoveryRecording> =
                                cr.recordings.into_iter().map(Into::into).collect();
                            let hub: Vec<DiscoveryRecording> =
                                core.into_iter().map(Into::into).collect();
                            (
                                hub,
                                Some(cr.playlist_title),
                                Some(cr.playlist_id),
                                Some(cr.playlist_count),
                                None,
                            )
                        }
                        Err(e) => (vec![], None, None, None, Some(e.to_string())),
                    }
                }
            },
            None => {
                logger::error(&format!("unknown recommendation type: {}", msg.rec_type));
                (
                    vec![],
                    None,
                    None,
                    None,
                    Some(format!("unknown recommendation type: {}", msg.rec_type)),
                )
            }
        };

        if error.is_none() {
            let pool = db.pool();
            for rec in &mut hub_recs {
                rec.is_owned =
                    duplicates::is_recording_owned(pool, Some(&rec.id), &rec.title, &rec.artist)
                        .await
                        .unwrap_or(false);
            }
        }

        GetLBRecommendationsResponse {
            id: msg.id,
            recommendations: hub_recs,
            playlist_title,
            playlist_id,
            playlist_count,
            error,
        }
        .send_signal_to_dart();
    }
}

pub async fn handle_validate_lb_token(context: Arc<AppContext>) {
    let receiver = ValidateLBTokenRequest::get_dart_signal_receiver();
    while let Some(signal_pack) = receiver.recv().await {
        let msg = signal_pack.message;

        match listenbrainz::validate_token(context.client(), &msg.token).await {
            Ok(v) => {
                ValidateLBTokenResponse {
                    id: msg.id,
                    valid: v.valid,
                    user_name: v.user_name,
                    message: v.message,
                }
                .send_signal_to_dart();
            }
            Err(e) => {
                logger::error(&format!("validate token failed: {}", e));
                ValidateLBTokenResponse {
                    id: msg.id,
                    valid: false,
                    user_name: None,
                    message: e.to_string(),
                }
                .send_signal_to_dart();
            }
        }
    }
}
