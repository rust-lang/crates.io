//! Endpoint for triggering a docs.rs rebuild

use super::update::authenticate;
use super::CrateVersionPath;
use crate::app::AppState;
use crate::controllers::helpers::ok_true;
use crate::rate_limiter::LimitedAction;
use crate::util::errors::{forbidden, AppResult};
use axum::response::Response;
use http::request::Parts;

/// Trigger a rebuild for the crate documentation on docs.rs.
#[utoipa::path(
    post,
    path = "/api/v1/crates/{name}/{version}/rebuild_docs",
    params(CrateVersionPath),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "versions",
    responses((status = 201, description = "Successful Response")),
)]
pub async fn rebuild_version_docs(
    app: AppState,
    path: CrateVersionPath,
    req: Parts,
) -> AppResult<Response> {
    let Some(ref docs_rs_api_token) = app.config.docs_rs_api_token else {
        return Err(forbidden("docs.rs integration is not configured"));
    };

    let mut conn = app.db_read().await?;
    // FIXME: which scope to use?
    let auth = authenticate(&req, &mut conn, &path.name).await?;

    // FIXME: rate limiting needed here? which Action?
    app.rate_limiter
        .check_rate_limit(auth.user_id(), LimitedAction::YankUnyank, &mut conn)
        .await?;

    let target_url = app
        .config
        .docs_rs_url
        .join(&format!("/crate/{}/{}/rebuild", path.name, path.version))
        .unwrap(); // FIXME: handle error

    let client = reqwest::Client::new();
    let response = client
        .post(target_url.as_str())
        .bearer_auth(docs_rs_api_token)
        .send()
        .await?;

    let status= = response.status();

    // reqwest::

    // perform_version_yank_update(
    //     &state,
    //     &mut conn,
    //     &mut version,
    //     &krate,
    //     &auth,
    //     Some(yanked),
    //     None,
    // )
    // .await?;

    ok_true()
}
