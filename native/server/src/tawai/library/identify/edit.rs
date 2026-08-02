use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tawai_core::audio;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct ReadFileTagsBody {
    pub path: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReadFileTagsResponseBody {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<String>,
    pub duration_secs: f64,
    pub error: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct WriteFileTagsBody {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genres: Vec<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct WriteFileTagsResponseBody {
    pub success: bool,
    pub error: Option<String>,
}

pub async fn handle_read_file_tags(
    _state: State<SharedState>,
    Json(body): Json<ReadFileTagsBody>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);

    match audio::tags::read_audio_tags(&path) {
        Ok((tag, duration_secs, _sample_rate, _bitrate)) => {
            let cover_base64 = tag
                .cover
                .as_ref()
                .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes));

            Json(ReadFileTagsResponseBody {
                title: tag.title,
                artist: tag.artist,
                album: tag.album,
                album_artist: tag.album_artist,
                genres: tag.genres,
                track_number: tag.track_number,
                disc_number: tag.disc_number,
                release_date: tag.release_date,
                lyrics: tag.lyrics,
                cover: cover_base64,
                duration_secs,
                error: None,
            })
            .into_response()
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("read_audio_tags failed: {e}"));
            Json(ReadFileTagsResponseBody {
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                album_artist: String::new(),
                genres: vec![],
                track_number: 0,
                disc_number: 0,
                release_date: None,
                lyrics: None,
                cover: None,
                duration_secs: 0.0,
                error: Some(e.to_string()),
            })
            .into_response()
        }
    }
}

pub async fn handle_write_file_tags(
    _state: State<SharedState>,
    Json(body): Json<WriteFileTagsBody>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);

    let cover_bytes = match &body.cover {
        Some(s) if !s.is_empty() => match base64::engine::general_purpose::STANDARD.decode(s) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(WriteFileTagsResponseBody {
                        success: false,
                        error: Some(format!("Invalid cover base64: {e}")),
                    }),
                )
                    .into_response();
            }
        },
        _ => None,
    };

    let tag = audio::tags::AudioTag {
        title: body.title,
        artist: body.artist,
        album: body.album,
        album_artist: body.album_artist,
        genres: body.genres,
        track_number: body.track_number,
        disc_number: body.disc_number,
        release_date: body.release_date,
        lyrics: body.lyrics,
        cover: cover_bytes,
        ..Default::default()
    };

    match audio::tags::write_audio_tags(&path, &tag) {
        Ok(()) => Json(WriteFileTagsResponseBody {
            success: true,
            error: None,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("write_audio_tags failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(WriteFileTagsResponseBody {
                    success: false,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}
