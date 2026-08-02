use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::metadata::musicbrainz;

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/identify/mb/release/{id}/tracks",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    params(
        ("id" = String, Path, description = "MusicBrainz release ID"),
    ),
    responses(
        (status = 200, description = "Release tracks", body = tawai_core::signals::metadata::ReleaseInfo),
    )
)]
pub async fn handle_get_release_tracks(
    State(state): State<SharedState>,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    match musicbrainz::fetch_release(state.context.client(), &release_id).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("fetch_release failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(tawai_core::signals::metadata::ReleaseInfo {
                    id: String::new(),
                    title: String::new(),
                    date: None,
                    country: None,
                    artist: String::new(),
                    artist_id: None,
                    tracks: vec![],
                    disambiguation: None,
                    total_discs: None,
                }),
            )
                .into_response()
        }
    }
}
