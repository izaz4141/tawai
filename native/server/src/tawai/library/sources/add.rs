use axum::{
    Json,
    extract::{Extension, State},
    response::IntoResponse,
};
use tawai_core::db::{account, library_source};
use tawai_core::signals::library::{AddLibrarySourceRequest, AddLibrarySourceResponse};

use crate::server::SharedState;

pub async fn handle_add_source(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<AddLibrarySourceRequest>,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let user = match account::get_user_by_id(db.pool(), &user_id, &mk).await {
        Ok(u) => u,
        Err(_) => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };

    match library_source::add_source(
        db.pool(),
        &user.id,
        &payload.url,
        &payload.name,
        &payload.source_type,
        "all",
    )
    .await
    {
        Ok(source_id) => Json(AddLibrarySourceResponse {
            id: payload.id,
            source_id,
            success: true,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!("add source failed: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(AddLibrarySourceResponse {
                    id: payload.id,
                    source_id: String::new(),
                    success: false,
                }),
            )
                .into_response()
        }
    }
}
