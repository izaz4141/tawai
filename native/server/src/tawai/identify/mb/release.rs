use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    metadata::musicbrainz,
    signals::metadata::{GetReleaseTracksResponse, ReleaseInfo},
};

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
        (status = 200, description = "Release tracks", body = GetReleaseTracksResponse),
    )
)]
pub async fn handle_get_release_tracks(
    State(state): State<SharedState>,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    match musicbrainz::fetch_release(state.context.client(), &release_id).await {
        Ok(data) => Json(get_release_tracks_response(release_id, data)).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("fetch_release failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetReleaseTracksResponse {
                    id: String::new(),
                    release_id,
                    release_title: String::new(),
                    release_date: None,
                    artist: String::new(),
                    artist_id: None,
                    disambiguation: None,
                    tracks: vec![],
                }),
            )
                .into_response()
        }
    }
}

fn get_release_tracks_response(release_id: String, info: ReleaseInfo) -> GetReleaseTracksResponse {
    GetReleaseTracksResponse {
        id: String::new(),
        release_id,
        release_title: info.title,
        release_date: info.date,
        artist: info.artist,
        artist_id: info.artist_id,
        disambiguation: info.disambiguation,
        tracks: info.tracks,
    }
}
