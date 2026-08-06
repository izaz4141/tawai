use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::db::account;
use tawai_core::signals::playback::{PlayTrackRequest, PlayTrackResponse};
use tawai_core::utils::playback::resolve_playable_track;

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/playback/play",
    tags = ["tawai.playback"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Playable track response"),
        (status = 404, description = "Track not found")
    )
)]
pub async fn handle_play_track(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Json(req): Json<PlayTrackRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let cfg = state.context.cfg().await;

    let mut result = resolve_playable_track(
        db.pool(),
        state.context.client(),
        req.track_id.as_deref(),
        req.track.as_ref().map(|t| t.title.as_str()),
        req.track.as_ref().map(|t| t.artists_string.as_str()),
        req.track.as_ref().map(|t| t.album_title.as_str()),
        req.track.as_ref().and_then(|t| t.mbid_recording.as_deref()),
        Some(&cfg),
    )
    .await;

    let file_path = if result.file_path.starts_with("http://")
        || result.file_path.starts_with("https://")
        || result.file_path.is_empty()
    {
        result.file_path
    } else if let Some(tid) = &result.resolved_track_id {
        let mk = state.context.master_key.read().await.clone();
        if let Ok(Some(api_key)) = account::get_user_api_key(db.pool(), &username, &mk).await {
            let mut headers = result.headers.unwrap_or_default();
            headers.push(("X-API-Key".to_string(), api_key));
            result.headers = Some(headers);
        }
        format!("/api/tawai/playback/stream/{}", tid)
    } else {
        result.file_path
    };

    let status = match &result.error {
        None => StatusCode::OK,
        Some(e) if e == "Track not found" || e == "No audio source found" => StatusCode::NOT_FOUND,
        Some(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (
        status,
        Json(PlayTrackResponse {
            id: req.id,
            file_path,
            error: result.error,
            headers: result.headers,
        }),
    )
        .into_response()
}
