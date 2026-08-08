use axum::{
    Extension, Json,
    extract::State,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::atomic::Ordering;
use tawai_core::signals::library::ScanProgress;
use tawai_core::{
    db::{account, library_source},
    utils::logger,
};
use tokio_stream::wrappers::WatchStream;
use utoipa::ToSchema;

use crate::server::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct ScanSourceQuery {
    pub source_id: String,
    pub force: Option<bool>,
}

fn sse_event(progress: ScanProgress) -> Result<Event, Infallible> {
    Ok(Event::default().json_data(progress).unwrap())
}

fn error_stream(msg: String) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(stream::once(async {
        sse_event(ScanProgress {
            complete: true,
            error: Some(msg),
            ..Default::default()
        })
    }))
}

#[utoipa::path(
    post,
    path = "/api/tawai/library/scan/source",
    tags = ["tawai.library"],
    security(("ApiKeyAuth" = [])),
    request_body = ScanSourceQuery,
    responses(
        (status = 200, description = "SSE stream of scan progress for a single source"),
    )
)]
pub async fn handle_scan_source(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(query): Json<ScanSourceQuery>,
) -> impl IntoResponse {
    let force = query.force.unwrap_or(false);

    if state.context.scan_running.swap(true, Ordering::SeqCst) {
        return error_stream("A scan is already in progress".to_string()).into_response();
    }

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let Ok(user) = account::get_user_by_id(db.pool(), &user_id, &mk).await else {
        logger::error(&format!("Cant find user with id: {}", user_id));
        state.context.scan_running.store(false, Ordering::SeqCst);
        return error_stream(format!("Cant find user with id: {}", user_id).to_string())
            .into_response();
    };

    // Enforce that the target source is accessible to the requesting user.
    let accessible = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
        .await
        .unwrap_or_default();
    let Some(source) = accessible.into_iter().find(|s| s.id == query.source_id) else {
        state.context.scan_running.store(false, Ordering::SeqCst);
        return error_stream("Library source is not accessible".to_string()).into_response();
    };

    let (tx, rx) = tokio::sync::watch::channel(ScanProgress::default());
    let rx2 = rx.clone();

    let client = state.context.client().clone();
    let sources = vec![source];
    let scan_ctx = state.context.clone();

    tokio::spawn(async move {
        let pool = db.pool();
        tawai_core::audio::scan::run_scan(&pool, client, &sources, force, Some(tx)).await;
        scan_ctx.scan_running.store(false, Ordering::SeqCst);
    });

    let progress_ctx = state.context.clone();
    tokio::spawn(async move {
        let mut rx = rx2;
        while rx.changed().await.is_ok() {
            let p = rx.borrow_and_update().clone();
            *progress_ctx.scan_progress.write().await = Some(p);
        }
    });

    let stream = WatchStream::new(rx).map(sse_event);

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
