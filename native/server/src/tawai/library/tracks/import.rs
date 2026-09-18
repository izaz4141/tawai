use axum::{
    Json,
    extract::{Extension, Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::{
    db::{account, library_source},
    utils::logger,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/tracks/import",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body(content = (), content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File imported into the source", body = serde_json::Value),
        (status = 400, description = "Import failed"),
    )
)]
pub async fn handle_import_track(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut source_id = String::new();
    let mut name = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name().unwrap_or("") {
            "source_id" => {
                if let Ok(text) = field.text().await {
                    source_id = text;
                }
            }
            "name" => {
                if let Ok(text) = field.text().await {
                    name = text;
                }
            }
            "file" => {
                if let Ok(data) = field.bytes().await {
                    bytes = data.to_vec();
                }
            }
            _ => {}
        }
    }

    if source_id.is_empty() || bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "Missing required field: source_id or file",
        )
            .into_response();
    }

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let Ok(user) = account::get_user_by_id(db.pool(), &user_id, &mk).await else {
        return (StatusCode::UNAUTHORIZED, "User not found").into_response();
    };

    let accessible = library_source::list_accessible_sources(db.pool(), &user.id, &user.role, &mk)
        .await
        .unwrap_or_default();
    let Some(source) = accessible.into_iter().find(|s| s.id == source_id) else {
        return (
            StatusCode::FORBIDDEN,
            "Library source is not accessible",
        )
            .into_response();
    };
    if !tawai_core::libsources::is_editable(&source.source_type) {
        return (
            StatusCode::BAD_REQUEST,
            format!("source is not editable: {}", source.source_type),
        )
            .into_response();
    }

    let filename = if name.is_empty() {
        "import.mp3".to_string()
    } else {
        name
    };

    match tawai_core::libsources::tawai::import_into_source(
        db.pool(),
        &source,
        &bytes,
        &filename,
        state.context.client(),
        &mk,
    )
    .await
    {
        Ok(file_path) => Json(serde_json::json!({
            "file_path": file_path,
            "filename": filename,
        }))
        .into_response(),
        Err(e) => {
            logger::error(&format!("tawai track import failed: {e}"));
            (
                StatusCode::BAD_REQUEST,
                format!("Import failed: {e}"),
            )
                .into_response()
        }
    }
}