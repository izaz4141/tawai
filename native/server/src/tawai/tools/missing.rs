use axum::{Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::{
    FindMissingMetadataRequest, FindMissingMetadataResponse, MissingMetadataCheck,
};

#[utoipa::path(
    post,
    path = "/api/tawai/tools/missing-metadata",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Tracks with missing metadata", body = FindMissingMetadataResponse)
    )
)]
pub async fn handle_find_missing_metadata(
    State(state): State<SharedState>,
    Json(query): Json<FindMissingMetadataRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let check = MissingMetadataCheck {
        check_title: query.check_title.unwrap_or(true),
        check_artist: query.check_artist.unwrap_or(true),
        check_album: query.check_album.unwrap_or(true),
        check_genre: query.check_genre.unwrap_or(true),
        check_year: query.check_year.unwrap_or(true),
        check_track_number: query.check_track_number.unwrap_or(true),
        check_cover: query.check_cover.unwrap_or(true),
    };

    match tawai_core::tools::missing::find_missing_metadata(db.pool(), &check).await {
        Ok(tracks) => Json(FindMissingMetadataResponse {
            tracks,
            error: None,
        }),
        Err(e) => Json(FindMissingMetadataResponse {
            tracks: vec![],
            error: Some(e.to_string()),
        }),
    }
}
