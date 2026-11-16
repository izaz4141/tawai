pub mod docs;
pub mod tawai;

use axum::Router;
use axum::middleware;
use axum::routing::get;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

use crate::utils::observability;
use crate::utils::server::SharedState;

pub fn create_router(state: SharedState) -> Router {
    let tawai_router = tawai::create_tawai_router(state.clone());
    let docs_router =
        docs::create_docs_router(state.clone()).layer(state.rate_limiters.global_layer());

    let status_router = Router::new()
        .route(
            "/api/tawai/system/status",
            get(tawai::system::status::handle_status),
        )
        .layer(state.rate_limiters.public_layer());

    Router::new()
        .nest("/api/tawai", tawai_router)
        .merge(docs_router)
        .merge(status_router)
        .layer(middleware::from_fn(observability::restore_request_uri))
        .layer(OtelInResponseLayer::default())
        .layer(middleware::from_fn(observability::access_log))
        .layer(OtelAxumLayer::default().filter(observability::should_trace))
        .layer(middleware::from_fn(observability::redact_request_uri))
        .with_state(state)
}
