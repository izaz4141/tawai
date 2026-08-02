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
pub struct ScanQuery {
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

pub async fn handle_scan(
    State(state): State<SharedState>,
    Extension(username): Extension<String>,
    Json(query): Json<ScanQuery>,
) -> impl IntoResponse {
    let force = query.force.unwrap_or(false);

    if state.context.scan_running.swap(true, Ordering::SeqCst) {
        return error_stream("A scan is already in progress".to_string()).into_response();
    }

    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let Ok(Some(user)) = account::get_user_by_username(db.pool(), &username, &mk).await else {
        logger::error(&format!("Cant find user with username: {}", username));
        return error_stream(format!("Cant find user with username: {}", username).to_string())
            .into_response();
    };

    let sources = library_source::list_accessible_sources(db.pool(), &user.id, &user.role)
        .await
        .unwrap_or_default();

    if sources.is_empty() {
        state.context.scan_running.store(false, Ordering::SeqCst);
        return error_stream("No library sources configured".to_string()).into_response();
    }

    let (tx, rx) = tokio::sync::watch::channel(ScanProgress::default());
    let rx2 = rx.clone();

    let client = state.context.client().clone();
    let sources2 = sources.clone();
    let ctx = state.context.clone();

    tokio::spawn(async move {
        let pool = db.pool();
        tawai_core::audio::scan::run_scan(&pool, client, &sources2, force, Some(tx)).await;
        ctx.scan_running.store(false, Ordering::SeqCst);
    });

    tokio::spawn(async move {
        let mut rx = rx2;
        while rx.changed().await.is_ok() {
            let p = rx.borrow_and_update().clone();
            *state.context.scan_progress.write().await = Some(p);
        }
    });

    let stream = WatchStream::new(rx).map(sse_event);

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
