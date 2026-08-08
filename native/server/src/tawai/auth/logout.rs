use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use time::Duration as TimeDuration;

use crate::server::SharedState;

fn remove_jwt_cookie(jar: CookieJar) -> CookieJar {
    let jwt = Cookie::build(("tawai_jwt", ""))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::ZERO)
        .build();
    let csrf = Cookie::build(("tawai_csrf", ""))
        .path("/")
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::ZERO)
        .build();
    jar.add(jwt).add(csrf)
}

#[utoipa::path(
    post,
    path = "/api/tawai/auth/logout",
    tags = ["tawai.auth"],
    security(("ApiKeyAuth" = [])),
    responses(
        (status = 200, description = "Logged out, cookies cleared"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn handle_logout(State(_state): State<SharedState>, jar: CookieJar) -> impl IntoResponse {
    (remove_jwt_cookie(jar), StatusCode::OK).into_response()
}
