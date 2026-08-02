use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::{metadata::musicbrainz, signals::metadata::RecordingInfo};
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct DiscoverSearchQuery {
    pub q: String,
}

#[utoipa::path(
    get,
    path = "/api/tawai/identify/mb/search",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    params(
        ("q" = String, Query, description = "Search query"),
    ),
    responses(
        (status = 200, description = "MusicBrainz search results", body = Vec<RecordingInfo>),
    )
)]
pub async fn handle_enhanced_search(
    State(state): State<SharedState>,
    Query(query): Query<DiscoverSearchQuery>,
) -> impl IntoResponse {
    match musicbrainz::lookup_by_query(state.context.client(), &query.q).await {
        Ok(data) => Json(data.recordings).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("search failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Vec::<RecordingInfo>::new()),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/tawai/identify/mb/recording/{mbid}",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    params(
        ("mbid" = String, Path, description = "MusicBrainz recording ID"),
    ),
    responses(
        (status = 200, description = "Recording details", body = RecordingInfo),
    )
)]
pub async fn handle_fetch_recording(
    State(state): State<SharedState>,
    Path(mbid): Path<String>,
) -> impl IntoResponse {
    match musicbrainz::fetch_recording(state.context.client(), &mbid).await {
        Ok(data) => Json(data).into_response(),
        Err(e) => {
            let empty = RecordingInfo {
                id: String::new(),
                title: String::new(),
                score: 0.0,
                artist: String::new(),
                artist_id: None,
                duration_secs: None,
                acoust_id: None,
                releases: vec![],
                cover: None,
            };
            tawai_core::utils::logger::error(&format!("fetch_recording failed: {e}"));
            (StatusCode::INTERNAL_SERVER_ERROR, Json(empty)).into_response()
        }
    }
}
