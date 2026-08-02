use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::dclient::iprev;
use tawai_core::signals::playback::PreviewTrackResponse;

use crate::server::SharedState;

pub async fn handle_preview_track(
    State(state): State<SharedState>,
    Extension(_username): Extension<String>,
    Json(req): Json<tawai_core::signals::playback::PreviewTrackRequest>,
) -> impl IntoResponse {
    match iprev::fetch_preview(
        state.context.client(),
        &req.track.artists_string,
        &req.track.title,
    )
    .await
    {
        Ok(Some(url)) => Json(PreviewTrackResponse {
            id: req.id,
            url: Some(url),
            source: Some("itunes".to_string()),
            error: None,
        })
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(PreviewTrackResponse {
                id: req.id,
                url: None,
                source: None,
                error: Some("No preview available".to_string()),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(PreviewTrackResponse {
                id: req.id,
                url: None,
                source: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}
