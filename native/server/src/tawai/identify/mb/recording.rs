use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    metadata::musicbrainz,
    signals::metadata::{EnhancedSearchRequest, EnhancedSearchResponse, FetchRecordingResponse},
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/identify/mb/search",
    tags = ["tawai.identify"],
    security(("ApiKeyAuth" = [])),
    params(
        ("query" = String, Query, description = "Search query"),
    ),
    responses(
        (status = 200, description = "MusicBrainz search results", body = EnhancedSearchResponse),
    )
)]
pub async fn handle_enhanced_search(
    State(state): State<SharedState>,
    Query(query): Query<EnhancedSearchRequest>,
) -> impl IntoResponse {
    match musicbrainz::lookup_by_query(state.context.client(), &query.query).await {
        Ok(data) => Json(EnhancedSearchResponse {
            id: String::new(),
            recordings: data.recordings,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("search failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(EnhancedSearchResponse {
                    id: String::new(),
                    recordings: vec![],
                }),
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
        (status = 200, description = "Recording details", body = FetchRecordingResponse),
    )
)]
pub async fn handle_fetch_recording(
    State(state): State<SharedState>,
    Path(mbid): Path<String>,
) -> impl IntoResponse {
    match musicbrainz::fetch_recording(state.context.client(), &mbid).await {
        Ok(data) => Json(FetchRecordingResponse {
            id: String::new(),
            recording: Some(data),
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("fetch_recording failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FetchRecordingResponse {
                    id: String::new(),
                    recording: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
