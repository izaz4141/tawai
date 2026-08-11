use crate::security::create_jwt_response;
use crate::server::{SharedState, build_jwt_cookie};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::extract::CookieJar;
use tawai_core::signals::account::ApiKeyResponse;

#[utoipa::path(
    get,
    path = "/api/tawai/auth/generate-api",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "API key generated successfully", body = ApiKeyResponse)
    )
)]
pub async fn handle_generate_api(
    State(state): State<SharedState>,
    Extension(user_id): Extension<String>,
    jar: CookieJar,
) -> impl IntoResponse {
    let db = state.context.db().await;
    let mk = state.context.master_key.read().await.clone();
    let new_key = tawai_core::db::account::regenerate_user_api_key(db.pool(), &user_id, &mk)
        .await
        .unwrap_or_default();

    let jwt_response = create_jwt_response(&state, &user_id).await.unwrap();
    let jar = build_jwt_cookie(jar, &jwt_response);

    let json_response = ApiKeyResponse {
        api_key: new_key,
        access_token: jwt_response.access_token,
        csrf_token: jwt_response.csrf_token,
        expires_in: jwt_response.expires_in,
    };
    (jar, Json(json_response)).into_response()
}
