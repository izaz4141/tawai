use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::atomic::Ordering;
use tawai_core::signals::library::{ScanLibraryRequest, ScanLibraryResponse, ScanProgress};
use tawai_core::{
    db::{account, library_source},
    utils::logger,
};

use crate::server::SharedState;

#[utoipa::path(
    post,
    path = "/api/tawai/library/scan",
    tags = ["tawai.library.scan"],
    security(("ApiKeyAuth" = [])),
    request_body = ScanLibraryRequest,
    responses(
        (status = 200, description = "Scan started", body = ScanLibraryResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn handle_scan(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(req): Json<ScanLibraryRequest>,
) -> impl IntoResponse {
    let id = req.id;
    let force = req.force;

    if state.context.scan_running.swap(true, Ordering::SeqCst) {
        return (
            StatusCode::OK,
            Json(ScanLibraryResponse {
                id,
                started: false,
                error: Some("A scan is already in progress".to_string()),
            }),
        )
            .into_response();
    }
    *state.context.scan_progress.write().await = None;

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let Ok(user) = account::get_user_by_id(db.pool(), &user_id, &mk).await else {
        state.context.scan_running.store(false, Ordering::SeqCst);
        logger::error(&format!("Cant find user with id: {}", user_id));
        return (
            StatusCode::OK,
            Json(ScanLibraryResponse {
                id,
                started: false,
                error: Some(format!("Cant find user with id: {}", user_id)),
            }),
        )
            .into_response();
    };

    let sources = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
        .await
        .unwrap_or_default();

    if sources.is_empty() {
        state.context.scan_running.store(false, Ordering::SeqCst);
        return (
            StatusCode::OK,
            Json(ScanLibraryResponse {
                id,
                started: false,
                error: Some("No library sources configured".to_string()),
            }),
        )
            .into_response();
    }

    let (tx, mut rx) = tokio::sync::watch::channel(ScanProgress::default());

    let client = state.context.client().clone();
    let sources2 = sources.clone();
    let ctx = state.context.clone();

    tokio::spawn(async move {
        tawai_core::audio::scan::run_scan(db.pool(), client, &sources2, force, Some(tx)).await;
        ctx.scan_running.store(false, Ordering::SeqCst);
    });

    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let p = rx.borrow_and_update().clone();
            *state.context.scan_progress.write().await = Some(p);
        }
    });

    (
        StatusCode::OK,
        Json(ScanLibraryResponse {
            id,
            started: true,
            error: None,
        }),
    )
        .into_response()
}
