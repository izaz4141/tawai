use crate::server::SharedState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use tawai_core::signals::account::{ListUsersResponse, UserListItem};
use tawai_core::utils::account::list_users;

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
    let mk = state.context.master_key.read().await.clone();

    let users = list_users(&db, &mk, false).await.unwrap_or_default();

    let response = ListUsersResponse {
        id: String::new(),
        users: users
            .into_iter()
            .map(|u| UserListItem {
                id: u.id,
                username: u.username,
                display_name: u.display_name,
                role: u.role,
                api_key: u.api_key,
            })
            .collect(),
    };

    Json(response).into_response()
}
