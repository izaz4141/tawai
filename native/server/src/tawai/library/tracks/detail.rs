use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use tawai_core::db::{library, library_source};
use tawai_core::signals::library::TrackInfo;
use tawai_core::utils::playback::resolve_track_source;

use crate::server::SharedState;

#[utoipa::path(
    get,
    path = "/api/tawai/library/tracks/{id}",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    params(
        ("id" = String, Path, description = "Track ID"),
    ),
    responses(
        (status = 200, description = "Track info", body = TrackInfo),
        (status = 404, description = "Track not found")
    )
)]
pub async fn handle_get_track(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::lookup_track(db.pool(), &id).await {
        Ok(Some(mut t)) => {
            let (source_type, source_url) =
                library_source::get_source_by_track_id(db.pool(), &t.id)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            let (resolved_path, _headers) = resolve_track_source(
                &t.file_path,
                &source_type,
                &source_url,
                state.context.client(),
            )
            .await;
            t.file_path = resolved_path;
            Json(t).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("get track failed: {}", e));
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn handle_track_cover(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::get_track_cover(db.pool(), &id).await {
        Ok(Some(cover)) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            );
            (StatusCode::OK, headers, cover).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("track cover failed: {}", e));
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
