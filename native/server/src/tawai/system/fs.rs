use crate::server::SharedState;
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use tawai_core::utils::fs::{FsListing, list_dir};
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDirQuery {
    /// Absolute path of the directory to list. Tilde (`~`) is expanded.
    #[serde(default)]
    pub path: String,
}

#[utoipa::path(
    get,
    path = "/api/tawai/system/fs/list",
    tags = ["tawai.system"],
    security(("ApiKeyAuth" = [])),
    params(ListDirQuery),
    responses(
        (status = 200, description = "Directory listing", body = FsListing),
        (status = 400, description = "Invalid or unreadable path")
    )
)]
pub async fn handle_list_dir(
    State(_state): State<SharedState>,
    Query(params): Query<ListDirQuery>,
) -> impl IntoResponse {
    match list_dir(&params.path) {
        Ok(listing) => Json(listing).into_response(),
        Err(e) => {
            tawai_core::utils::logger::debug(&format!("fs list failed: {e}"));
            (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid or unreadable path",
            )
                .into_response()
        }
    }
}
