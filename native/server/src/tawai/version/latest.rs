use crate::server::SharedState;
use axum::Json;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use tawai_core::signals::version::{GetLatestVersionRequest, GetLatestVersionResponse};

#[utoipa::path(
    get,
    path = "/api/tawai/version/latest",
    tags = ["tawai.version"],
    security(("ApiKeyAuth" = [])),
    params(
        ("owner" = String, Query, description = "Repository owner"),
        ("repo" = String, Query, description = "Repository name"),
        ("nightly" = bool, Query, description = "Check nightly releases"),
        ("atomic" = bool, Query, description = "Use Atom feed"),
    ),
    responses(
        (status = 200, description = "Latest version info", body = GetLatestVersionResponse),
    )
)]
pub async fn handle_version_latest(
    State(state): State<SharedState>,
    Query(params): Query<GetLatestVersionRequest>,
) -> impl IntoResponse {
    let repo_owner = params.owner;
    let repo_name = params.repo;
    let check_nightly = params.nightly;
    let use_atom = params.atomic;

    match tawai_core::utils::version::get_latest_version(
        state.context.client(),
        &repo_owner,
        &repo_name,
        check_nightly,
        use_atom,
    )
    .await
    {
        Ok(info) => Json(GetLatestVersionResponse {
            id: String::new(),
            version: Some(info.version),
            tag_name: Some(info.tag_name),
            release_notes: Some(info.release_notes),
            published_at: Some(info.published_at),
            error: info.error,
        })
        .into_response(),
        Err(e) => {
            tawai_core::utils::logger::error(&format!(
                "Error getting latest {}/{}: {:#?}",
                &repo_owner, &repo_name, &e
            ));
            (
                StatusCode::BAD_REQUEST,
                Json(GetLatestVersionResponse {
                    id: String::new(),
                    version: None,
                    tag_name: None,
                    release_notes: None,
                    published_at: None,
                    error: Some(e),
                }),
            )
                .into_response()
        }
    }
}
