use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use tawai_core::db::library;

use crate::server::SharedState;

pub async fn handle_album_cover(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match library::get_album_cover(db.pool(), &id).await {
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
            tawai_core::utils::logger::error(&format!("album cover failed: {}", e));
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
