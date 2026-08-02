use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::{metadata::lrclib, signals::metadata::LyricsResult};
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct GetLyricsQuery {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<f64>,
    #[serde(default = "default_prefer_sync")]
    pub prefer_sync: bool,
}

fn default_prefer_sync() -> bool {
    true
}

#[derive(Deserialize, ToSchema)]
pub struct SearchLyricsQuery {
    pub q: String,
}

#[utoipa::path(
    get,
    path = "/api/tawai/library/identify/lyrics",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("title" = String, Query, description = "Track title"),
        ("artist" = String, Query, description = "Artist name"),
        ("album" = Option<String>, Query, description = "Album name"),
        ("duration" = Option<f64>, Query, description = "Track duration in seconds"),
        ("prefer_sync" = bool, Query, description = "Prefer synced lyrics"),
    ),
    responses(
        (status = 200, description = "Lyrics result", body = LyricsResult),
        (status = 404, description = "Not found"),
    )
)]
pub async fn handle_get_lyrics(
    State(state): State<SharedState>,
    Query(query): Query<GetLyricsQuery>,
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
    params(
        ("q" = String, Query, description = "Search query"),
    ),
    responses(
        (status = 200, description = "Search results", body = Vec<LyricsResult>),
    )
)]
pub async fn handle_search_lyrics(
    State(state): State<SharedState>,
    Query(query): Query<SearchLyricsQuery>,
) -> impl IntoResponse {
    match lrclib::search_lyrics(state.context.client(), &query.q).await {
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
