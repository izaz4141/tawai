pub mod docs;
pub mod tawai;

use axum::Router;
use axum::routing::get;

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
        .with_state(state)
}
