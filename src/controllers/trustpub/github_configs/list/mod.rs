use crate::app::AppState;
use crate::auth::{CookieCredentials, Permission};
use crate::controllers::krate::load_crate;
use crate::controllers::trustpub::github_configs::json::{self, ListResponse};
use crate::util::errors::AppResult;
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use crates_io_database::models::trustpub::GitHubConfig;
use crates_io_database::schema::trustpub_configs_github;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;

#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct ListQueryParams {
    /// Name of the crate to list Trusted Publishing configurations for.
    #[serde(rename = "crate")]
    pub krate: String,
}

/// List Trusted Publishing configurations for GitHub Actions.
#[utoipa::path(
    get,
    path = "/api/v1/trusted_publishing/github_configs",
    params(ListQueryParams),
    security(("cookie" = [])),
    tag = "trusted_publishing",
    responses((status = 200, description = "Successful Response", body = inline(ListResponse))),
)]
pub async fn list_trustpub_github_configs(
    state: AppState,
    params: ListQueryParams,
    creds: CookieCredentials,
    parts: Parts,
) -> AppResult<Json<ListResponse>> {
    let mut conn = state.db_read().await?;

    let krate = load_crate(&mut conn, &params.krate).await?;

    let permission = Permission::ListTrustPubGitHubConfigs { krate: &krate };
    creds.validate(&mut conn, &parts, permission).await?;

    let configs = trustpub_configs_github::table
        .filter(trustpub_configs_github::crate_id.eq(krate.id))
        .select(GitHubConfig::as_select())
        .load::<GitHubConfig>(&mut conn)
        .await?;

    let github_configs = configs
        .into_iter()
        .map(|config| json::GitHubConfig {
            id: config.id,
            krate: krate.name.clone(),
            repository_owner: config.repository_owner,
            repository_owner_id: config.repository_owner_id,
            repository_name: config.repository_name,
            workflow_filename: config.workflow_filename,
            environment: config.environment,
            created_at: config.created_at,
        })
        .collect();

    Ok(Json(ListResponse { github_configs }))
}
