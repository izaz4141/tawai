use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};

use tawai_core::{
    audio,
    db::library,
    signals::metadata::{
        ReadFileTagsBytesRequest, ReadFileTagsRequest, ReadFileTagsResponse,
        WriteFileTagsBytesRequest, WriteFileTagsBytesResponse, WriteFileTagsRequest,
        WriteFileTagsResponse,
    },
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/read",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = ReadFileTagsRequest,
    responses(
        (status = 200, description = "File tags read", body = ReadFileTagsResponse),
        (status = 500, description = "Error reading tags")
    )
)]
pub async fn handle_read_file_tags(
    _state: State<SharedState>,
    Json(body): Json<ReadFileTagsRequest>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);

    match audio::tags::read_audio_tags(&path) {
        Ok((tag, duration_secs, _sample_rate, _bitrate)) => Json(
            ReadFileTagsResponse::from_audio_tag(body.id, &tag, duration_secs),
        )
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("read_audio_tags failed: {e}"));
            Json(ReadFileTagsResponse {
                id: body.id,
                error: Some(e.to_string()),
                ..Default::default()
            })
            .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/write",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = WriteFileTagsRequest,
    responses(
        (status = 200, description = "File tags written", body = WriteFileTagsResponse),
        (status = 400, description = "Error writing tags")
    )
)]
pub async fn handle_write_file_tags(
    state: State<SharedState>,
    Json(body): Json<WriteFileTagsRequest>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&body.path);
    let sync_database = body.sync_database;

    let tag = audio::tags::AudioTag {
        title: body.title,
        artist: body.artist,
        artists: body.artists,
        album: body.album,
        album_artist: body.album_artist,
        album_artists: body.album_artists,
        genres: body.genres,
        track_number: body.track_number,
        disc_number: body.disc_number,
        total_tracks: body.total_tracks,
        total_discs: body.total_discs,
        release_date: body.release_date,
        original_date: body.original_date,
        media: body.media,
        script: body.script,
        record_label: body.record_label,
        release_country: body.release_country,
        release_type: body.release_type,
        isrc: body.isrc,
        mbid_recording: body.mbid_recording,
        mbid_track: body.mbid_track,
        mbid_artist: body.mbid_artist,
        mbid_release: body.mbid_release,
        mbid_release_artist: body.mbid_release_artist,
        mbid_release_group: body.mbid_release_group,
        artist_sort: body.artist_sort,
        album_artist_sort: body.album_artist_sort,
        comment: body.comment,
        copyright: body.copyright,
        lyrics: body.lyrics,
        cover: body.cover,
        ..Default::default()
    };

    match audio::tags::write_audio_tags(
        &path,
        &tag,
        audio::tags::TagWritePolicy::replace(body.clear_cover),
    ) {
        Ok(()) => {
            let db = state.context.db().await;
            // Mirror the edit to the remote tawai source (best-effort).
            let mk = state.context.master_key.read().await.clone();
            if let Err(e) = tawai_core::libsources::tawai::mirror_tag_write(
                db.pool(),
                &path.to_string_lossy(),
                &tag,
                body.clear_cover,
                state.context.client(),
                &mk,
            )
            .await
            {
                tawai_core::utils::logger::warn(&format!("tawai tag mirror failed: {e}"));
            }
            // Reflect the edit in this library's row so it does not wait for a
            // rescan. Best-effort: the write already succeeded.
            let sync_ok = if sync_database {
                match library::sync_track_tags_by_file_path(
                    db.pool(),
                    &body.path,
                    &tag,
                    body.clear_cover,
                )
                .await
                {
                    Ok(_) => true,
                    Err(e) => {
                        tawai_core::utils::logger::warn(&format!(
                            "failed to sync library row after tag write for {}: {e}",
                            body.path
                        ));
                        false
                    }
                }
            } else {
                true
            };
            // Writing tags rewrote the file's bytes, so the stored SHA-256 is
            // now stale. Refresh it only once the row's tag metadata is in
            // place (or when DB sync is disabled), so a failed sync leaves the
            // row in its prior, self-consistent state instead of pairing stale
            // tags with a fresh hash.
            if sync_ok {
                if let Ok(hash) = tawai_core::libsources::local::hash_file(&path) {
                    if let Err(e) =
                        library::update_file_hash_by_path(db.pool(), &path.to_string_lossy(), &hash)
                            .await
                    {
                        tawai_core::utils::logger::warn(&format!(
                            "failed to update stored file_hash after tag write for {}: {}",
                            path.display(),
                            e
                        ));
                    }
                } else {
                    tawai_core::utils::logger::warn(&format!(
                        "failed to compute file_hash after tag write for {}",
                        path.display()
                    ));
                }
            }
            Json(WriteFileTagsResponse {
                id: body.id,
                success: true,
                error: None,
            })
            .into_response()
        }
        Err(e) => {
            tawai_core::utils::logger::error(&format!("write_audio_tags failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(WriteFileTagsResponse {
                    id: body.id,
                    success: false,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/read-bytes",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body(content = ReadFileTagsBytesRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File tags read from bytes", body = ReadFileTagsResponse),
        (status = 400, description = "Error reading tags")
    )
)]
pub async fn handle_read_file_tags_bytes(
    _state: State<SharedState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut filename = String::new();
    let mut bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name() {
            Some("filename") => {
                if let Ok(text) = field.text().await {
                    filename = text;
                }
            }
            Some("file") => {
                if let Ok(data) = field.bytes().await {
                    bytes = data.to_vec();
                }
            }
            _ => {}
        }
    }

    if filename.is_empty() || bytes.is_empty() {
        return Json(ReadFileTagsResponse {
            error: Some("Missing filename or file".to_string()),
            ..Default::default()
        })
        .into_response();
    }

    match audio::tags::read_audio_tags_from_bytes(&filename, &bytes) {
        Ok((tag, duration_secs, _sample_rate, _bitrate)) => Json(
            ReadFileTagsResponse::from_audio_tag(String::new(), &tag, duration_secs),
        )
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("read_audio_tags_from_bytes failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(ReadFileTagsResponse {
                    error: Some(e.to_string()),
                    ..Default::default()
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/identify/tags/write-bytes",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body(content = WriteFileTagsBytesRequest, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File tags written; returns the modified file bytes", content_type = "application/octet-stream"),
        (status = 400, description = "Error writing tags", body = WriteFileTagsBytesResponse)
    )
)]
pub async fn handle_write_file_tags_bytes(
    _state: State<SharedState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut id = String::new();
    let mut filename = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    let mut title = String::new();
    let mut artist = String::new();
    let mut artists: Vec<String> = Vec::new();
    let mut album = String::new();
    let mut album_artist = String::new();
    let mut album_artists: Vec<String> = Vec::new();
    let mut genres: Vec<String> = Vec::new();
    let mut track_number = 0i32;
    let mut disc_number = 0i32;
    let mut total_tracks = 0i32;
    let mut total_discs = 0i32;
    let mut release_date: Option<String> = None;
    let mut original_date: Option<String> = None;
    let mut media: Option<String> = None;
    let mut script: Option<String> = None;
    let mut record_label: Option<String> = None;
    let mut release_country: Option<String> = None;
    let mut release_type: Vec<String> = Vec::new();
    let mut isrc: Option<String> = None;
    let mut mbid_recording: Option<String> = None;
    let mut mbid_track: Option<String> = None;
    let mut mbid_artist: Option<String> = None;
    let mut mbid_release: Option<String> = None;
    let mut mbid_release_artist: Option<String> = None;
    let mut mbid_release_group: Option<String> = None;
    let mut artist_sort = String::new();
    let mut album_artist_sort = String::new();
    let mut comment: Option<String> = None;
    let mut copyright: Option<String> = None;
    let mut lyrics: Option<String> = None;
    let mut cover: Option<Vec<u8>> = None;
    let mut clear_cover = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name().unwrap_or("") {
            "id" => {
                if let Ok(text) = field.text().await {
                    id = text;
                }
            }
            "filename" => {
                if let Ok(text) = field.text().await {
                    filename = text;
                }
            }
            "file" => {
                if let Ok(data) = field.bytes().await {
                    bytes = data.to_vec();
                }
            }
            "title" => {
                if let Ok(text) = field.text().await {
                    title = text;
                }
            }
            "artist" => {
                if let Ok(text) = field.text().await {
                    artist = text;
                }
            }
            "artists" => {
                if let Ok(text) = field.text().await {
                    artists.extend(split_multi(&text));
                }
            }
            "album" => {
                if let Ok(text) = field.text().await {
                    album = text;
                }
            }
            "album_artist" => {
                if let Ok(text) = field.text().await {
                    album_artist = text;
                }
            }
            "album_artists" => {
                if let Ok(text) = field.text().await {
                    album_artists.extend(split_multi(&text));
                }
            }
            "genres" => {
                if let Ok(text) = field.text().await {
                    genres.extend(split_multi(&text));
                }
            }
            "track_number" => {
                if let Ok(text) = field.text().await {
                    track_number = text.parse().unwrap_or(0);
                }
            }
            "total_tracks" => {
                if let Ok(text) = field.text().await {
                    total_tracks = text.parse().unwrap_or(0);
                }
            }
            "disc_number" => {
                if let Ok(text) = field.text().await {
                    disc_number = text.parse().unwrap_or(0);
                }
            }
            "total_discs" => {
                if let Ok(text) = field.text().await {
                    total_discs = text.parse().unwrap_or(0);
                }
            }
            "release_date" => {
                if let Ok(text) = field.text().await {
                    release_date = if text.is_empty() { None } else { Some(text) };
                }
            }
            "original_date" => {
                if let Ok(text) = field.text().await {
                    original_date = if text.is_empty() { None } else { Some(text) };
                }
            }
            "media" => {
                if let Ok(text) = field.text().await {
                    media = if text.is_empty() { None } else { Some(text) };
                }
            }
            "script" => {
                if let Ok(text) = field.text().await {
                    script = if text.is_empty() { None } else { Some(text) };
                }
            }
            "record_label" => {
                if let Ok(text) = field.text().await {
                    record_label = if text.is_empty() { None } else { Some(text) };
                }
            }
            "release_country" => {
                if let Ok(text) = field.text().await {
                    release_country = if text.is_empty() { None } else { Some(text) };
                }
            }
            "release_type" => {
                if let Ok(text) = field.text().await {
                    release_type.extend(split_multi(&text));
                }
            }
            "isrc" => {
                if let Ok(text) = field.text().await {
                    isrc = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_recording" => {
                if let Ok(text) = field.text().await {
                    mbid_recording = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_track" => {
                if let Ok(text) = field.text().await {
                    mbid_track = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_artist" => {
                if let Ok(text) = field.text().await {
                    mbid_artist = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_release" => {
                if let Ok(text) = field.text().await {
                    mbid_release = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_release_artist" => {
                if let Ok(text) = field.text().await {
                    mbid_release_artist = if text.is_empty() { None } else { Some(text) };
                }
            }
            "mbid_release_group" => {
                if let Ok(text) = field.text().await {
                    mbid_release_group = if text.is_empty() { None } else { Some(text) };
                }
            }
            "artist_sort" => {
                if let Ok(text) = field.text().await {
                    artist_sort = text;
                }
            }
            "album_artist_sort" => {
                if let Ok(text) = field.text().await {
                    album_artist_sort = text;
                }
            }
            "comment" => {
                if let Ok(text) = field.text().await {
                    comment = if text.is_empty() { None } else { Some(text) };
                }
            }
            "copyright" => {
                if let Ok(text) = field.text().await {
                    copyright = if text.is_empty() { None } else { Some(text) };
                }
            }
            "lyrics" => {
                if let Ok(text) = field.text().await {
                    lyrics = if text.is_empty() { None } else { Some(text) };
                }
            }
            "cover" => {
                if let Ok(data) = field.bytes().await {
                    cover = Some(data.to_vec());
                }
            }
            "clear_cover" => {
                if let Ok(text) = field.text().await {
                    clear_cover = text.trim().parse().unwrap_or(false);
                }
            }
            _ => {}
        }
    }

    if filename.is_empty() || bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(WriteFileTagsBytesResponse {
                id,
                success: false,
                bytes: vec![],
                error: Some("Missing filename or file".to_string()),
            }),
        )
            .into_response();
    }

    let tag = audio::tags::AudioTag {
        title,
        artist,
        artists,
        album,
        album_artist,
        album_artists,
        genres,
        track_number,
        disc_number,
        total_tracks,
        total_discs,
        release_date,
        original_date,
        media,
        script,
        record_label,
        release_country,
        release_type,
        isrc,
        mbid_recording,
        mbid_track,
        mbid_artist,
        mbid_release,
        mbid_release_artist,
        mbid_release_group,
        artist_sort,
        album_artist_sort,
        comment,
        copyright,
        lyrics,
        cover,
        ..Default::default()
    };

    match audio::tags::write_audio_tags_to_bytes(
        &filename,
        &bytes,
        &tag,
        audio::tags::TagWritePolicy::replace(clear_cover),
    ) {
        Ok(new_bytes) => (StatusCode::OK, new_bytes).into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("write_audio_tags_to_bytes failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                Json(WriteFileTagsBytesResponse {
                    id,
                    success: false,
                    bytes: vec![],
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

/// Split a `; `-separated multi-value wire field into individual values.
fn split_multi(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
