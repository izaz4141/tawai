use axum::{Json, extract::State, response::IntoResponse};

use crate::server::SharedState;
use tawai_core::signals::tools::{
    RomajizeLyricsRequest, RomajizeLyricsResponse, WriteTrackLyricsRequest,
    WriteTrackLyricsResponse,
};

#[utoipa::path(
    post,
    path = "/api/tawai/tools/romajize-lyrics",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Romajized lyrics result", body = RomajizeLyricsResponse)
    )
)]
pub async fn handle_romajize_lyrics(Json(body): Json<RomajizeLyricsRequest>) -> impl IntoResponse {
    match tawai_core::tools::lyrics::romajize_lyrics(body) {
        Ok(result) => Json(RomajizeLyricsResponse {
            romajized: result.romajized,
            synced: result.synced,
            error: result.error,
        }),
        Err(e) => Json(RomajizeLyricsResponse {
            romajized: String::new(),
            synced: false,
            error: Some(e.to_string()),
        }),
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/tools/write-lyrics",
    tag = "tawai.tools",
    responses(
        (status = 200, description = "Write lyrics result", body = WriteTrackLyricsResponse)
    )
)]
pub async fn handle_write_track_lyrics(
    State(state): State<SharedState>,
    Json(body): Json<WriteTrackLyricsRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    match tawai_core::tools::lyrics::write_track_lyrics(db.pool(), &body.track_id, &body.lyrics)
        .await
    {
        Ok(result) => Json(WriteTrackLyricsResponse {
            success: result.success,
            error: result.error,
        }),
        Err(e) => Json(WriteTrackLyricsResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}
