use crate::server::SharedState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Serialize, ToSchema)]
pub struct ListUsersResponse {
    pub users: Vec<UserInfo>,
}

#[utoipa::path(
    get,
    path = "/api/tawai/account/list",
    tags = ["tawai.account"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "List of all users", body = ListUsersResponse)
    )
)]
pub async fn handle_list_users(
    State(state): State<SharedState>,
    Extension(_username): Extension<String>,
) -> impl IntoResponse {
    let db = state.context.db().await;

    let users = tawai_core::db::account::get_all_users(db.pool())
        .await
        .unwrap_or_default();

    let response = ListUsersResponse {
        users: users
            .into_iter()
            .map(|u| UserInfo {
                id: u.id,
                username: u.username,
                display_name: u.display_name,
                role: u.role,
            })
            .collect(),
    };

    Json(response).into_response()
}
