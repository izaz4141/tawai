use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::utils::playback::resolve_playable_track;
use tawai_core::{
    signals::playback::{PlayTrackRequest, PlayTrackResponse},
    utils::logger,
};

use crate::security::create_jwt_response;
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
    Extension(user_id): Extension<String>,
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
        match create_jwt_response(&state, &user_id).await {
            Ok(jwt_resp) => format!(
                "/api/tawai/playback/stream/{}?token={}",
                tid, jwt_resp.access_token
            ),
            Err(e) => {
                logger::error(&format!("Error creating jwt_response: {e}"));
                result.error = Some("Error creating jwt_response".to_string());
                result.file_path
            }
        }
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
