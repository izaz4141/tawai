use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    metadata::lrclib,
    signals::metadata::{FetchLyricsRequest, LyricsResult, SearchLyricsRequest},
};

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/lyrics",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Lyrics result", body = LyricsResult),
        (status = 404, description = "Not found"),
    )
)]
pub async fn handle_get_lyrics(
    State(state): State<SharedState>,
    Query(query): Query<FetchLyricsRequest>,
) -> impl IntoResponse {
    let duration = query.duration.unwrap_or(0.0);
    let album = query.album.as_deref().unwrap_or("");

    match lrclib::get_lyrics(
        state.context.client(),
        &query.title,
        &query.artist,
        album,
        duration,
        query.prefer_sync,
    )
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get_lyrics failed: {e}"));
            (
                StatusCode::NOT_FOUND,
                Json(LyricsResult {
                    id: 0,
                    title: String::new(),
                    artist: String::new(),
                    album: String::new(),
                    duration: 0.0,
                    instrumental: false,
                    lyrics: String::new(),
                    synced: false,
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/lyrics/search",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Search results", body = Vec<LyricsResult>),
    )
)]
pub async fn handle_search_lyrics(
    State(state): State<SharedState>,
    Query(query): Query<SearchLyricsRequest>,
) -> impl IntoResponse {
    match lrclib::search_lyrics(state.context.client(), &query.query, query.prefer_sync).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("search_lyrics failed: {e}"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Vec::<LyricsResult>::new()),
            )
                .into_response()
        }
    }
}
