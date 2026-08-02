use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use tawai_core::audio::tags::AudioTag;
use tawai_core::tools::rename::format_naming_pattern;

use crate::server::SharedState;

#[derive(Deserialize)]
pub struct FormatNamingPreviewBody {
    pub pattern: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub release_date: Option<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub album_disambiguation: Option<String>,
    pub total_discs: i32,
}

pub async fn handle_format_naming_preview(
    State(_state): State<SharedState>,
    Json(body): Json<FormatNamingPreviewBody>,
) -> impl IntoResponse {
    let tag = AudioTag {
        title: body.title,
        artist: body.artist,
        album_artist: body.album_artist,
        album: body.album,
        release_date: body.release_date,
        track_number: body.track_number,
        disc_number: body.disc_number,
        album_disambiguation: body.album_disambiguation,
        total_discs: body.total_discs,
        ..Default::default()
    };

    let result = format_naming_pattern(&body.pattern, &tag);
    Json(serde_json::json!({ "result": result }))
}
