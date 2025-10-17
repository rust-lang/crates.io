use super::json;
use crate::app::AppState;
use crate::util::errors::{AppResult, BoxedAppError, bad_request, server_error};
use axum::Json;
use chrono::{DateTime, Utc};
use crates_io_database::models::trustpub::{
    GitHubConfig, GitLabConfig, NewToken, NewUsedJti, TrustpubData,
};
use crates_io_database::schema::{trustpub_configs_github, trustpub_configs_gitlab};
use crates_io_diesel_helpers::lower;
use crates_io_trustpub::access_token::AccessToken;
use crates_io_trustpub::github::{GITHUB_ISSUER_URL, GitHubClaims};
use crates_io_trustpub::gitlab::{GITLAB_ISSUER_URL, GitLabClaims};
use crates_io_trustpub::keystore::DecodingKey;
use crates_io_trustpub::unverified::UnverifiedClaims;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use secrecy::ExposeSecret;
use tracing::warn;

#[cfg(test)]
mod github_tests;
#[cfg(test)]
mod gitlab_tests;

/// Exchange an OIDC token for a temporary access token.
#[utoipa::path(
    post,
    path = "/api/v1/trusted_publishing/tokens",
    request_body = inline(json::ExchangeRequest),
    tag = "trusted_publishing",
    responses((status = 200, description = "Successful Response", body = inline(json::ExchangeResponse))),
)]
pub async fn exchange_trustpub_token(
    state: AppState,
    json: json::ExchangeRequest,
) -> AppResult<Json<json::ExchangeResponse>> {
    let unverified_jwt = json.jwt;

    let unverified_token_data = UnverifiedClaims::decode(&unverified_jwt)
        .map_err(|_err| bad_request("Failed to decode JWT"))?;

    let unverified_issuer = unverified_token_data.claims.iss;
    let Some(keystore) = state.oidc_key_stores.get(&unverified_issuer) else {
        return Err(unsupported_issuer(&unverified_issuer));
    };

    let Some(unverified_key_id) = unverified_token_data.header.kid else {
        let message = "Missing JWT key ID";
        return Err(bad_request(message));
    };

    let key = match keystore.get_oidc_key(&unverified_key_id).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            return Err(bad_request("Invalid JWT key ID"));
        }
        Err(err) => {
            warn!("Failed to load OIDC key set: {err}");
            return Err(server_error("Failed to load OIDC key set"));
        }
    };

    match unverified_issuer.as_str() {
        GITHUB_ISSUER_URL => handle_github_token(&state, &unverified_jwt, &key).await,
        GITLAB_ISSUER_URL => handle_gitlab_token(&state, &unverified_jwt, &key).await,
        _ => Err(unsupported_issuer(&unverified_issuer)),
    }
}

fn unsupported_issuer(issuer: &str) -> BoxedAppError {
    bad_request(format!("Unsupported JWT issuer: {issuer}"))
}

async fn insert_jti(conn: &mut AsyncPgConnection, jti: &str, exp: DateTime<Utc>) -> AppResult<()> {
    let used_jti = NewUsedJti::new(jti, exp);
    match used_jti.insert(conn).await {
        Ok(_) => Ok(()), // JTI was successfully inserted, continue
        Err(DatabaseError(UniqueViolation, _)) => {
            warn!("Attempted JWT reuse (jti: {jti})");
            let detail = "JWT has already been used";
            Err(bad_request(detail))
        }
        Err(err) => Err(err.into()),
    }
}

async fn handle_github_token(
    state: &AppState,
    unverified_jwt: &str,
    key: &DecodingKey,
) -> AppResult<Json<json::ExchangeResponse>> {
    let audience = &state.config.trustpub_audience;
    let signed_claims = GitHubClaims::decode(unverified_jwt, audience, key).map_err(|err| {
        warn!("Failed to decode JWT: {err}");
        bad_request("Failed to decode JWT")
    })?;

    let mut conn = state.db_write().await?;

    conn.transaction(|conn| Box::pin(handle_github_token_inner(conn, signed_claims)))
        .await
}

async fn handle_github_token_inner(
    conn: &mut AsyncPgConnection,
    signed_claims: GitHubClaims,
) -> AppResult<Json<json::ExchangeResponse>> {
    insert_jti(conn, &signed_claims.jti, signed_claims.exp).await?;

    let repo = &signed_claims.repository;
    let Some((repository_owner, repository_name)) = repo.split_once('/') else {
        warn!("Unexpected repository format in JWT: {repo}");
        let message = "Unexpected `repository` value";
        return Err(bad_request(message));
    };

    let Some(workflow_filename) = signed_claims.workflow_filename() else {
        let workflow_ref = &signed_claims.workflow_ref;
        warn!("Unexpected `workflow_ref` format in JWT: {workflow_ref}");
        let message = "Unexpected `workflow_ref` value";
        return Err(bad_request(message));
    };

    let Ok(repository_owner_id) = signed_claims.repository_owner_id.parse::<i32>() else {
        let repository_owner_id = &signed_claims.repository_owner_id;
        warn!("Unexpected `repository_owner_id` format in JWT: {repository_owner_id}");
        let message = "Unexpected `repository_owner_id` value";
        return Err(bad_request(message));
    };

    let mut repo_configs = GitHubConfig::query()
        .filter(lower(trustpub_configs_github::repository_owner).eq(lower(&repository_owner)))
        .filter(lower(trustpub_configs_github::repository_name).eq(lower(&repository_name)))
        .load(conn)
        .await?;

    if repo_configs.is_empty() {
        let message = format!("No Trusted Publishing config found for repository `{repo}`.");
        return Err(bad_request(message));
    }

    let mismatched_owner_ids: Vec<String> = repo_configs
        .extract_if(.., |config| {
            config.repository_owner_id != repository_owner_id
        })
        .map(|config| config.repository_owner_id.to_string())
        .collect();

    if repo_configs.is_empty() {
        let message = format!(
            "The Trusted Publishing config for repository `{repo}` does not match the repository owner ID ({repository_owner_id}) in the JWT. Expected owner IDs: {}. Please recreate the Trusted Publishing config to update the repository owner ID.",
            mismatched_owner_ids.join(", ")
        );
        return Err(bad_request(message));
    }

    let mismatched_workflows: Vec<String> = repo_configs
        .extract_if(.., |config| config.workflow_filename != workflow_filename)
        .map(|config| format!("`{}`", config.workflow_filename))
        .collect();

    if repo_configs.is_empty() {
        let message = format!(
            "The Trusted Publishing config for repository `{repo}` does not match the workflow filename `{workflow_filename}` in the JWT. Expected workflow filenames: {}",
            mismatched_workflows.join(", ")
        );
        return Err(bad_request(message));
    }

    let mismatched_environments: Vec<String> = repo_configs
        .extract_if(.., |config| {
            match (&config.environment, &signed_claims.environment) {
                // Keep configs with no environment requirement
                (None, _) => false,
                // Remove configs requiring environment when JWT has none
                (Some(_), None) => true,
                // Remove non-matching environments
                (Some(config_env), Some(signed_env)) => {
                    config_env.to_lowercase() != signed_env.to_lowercase()
                }
            }
        })
        .filter_map(|config| config.environment.map(|env| format!("`{env}`")))
        .collect();

    if repo_configs.is_empty() {
        let message = if let Some(signed_environment) = &signed_claims.environment {
            format!(
                "The Trusted Publishing config for repository `{repo}` does not match the environment `{signed_environment}` in the JWT. Expected environments: {}",
                mismatched_environments.join(", ")
            )
        } else {
            format!(
                "The Trusted Publishing config for repository `{repo}` requires an environment, but the JWT does not specify one. Expected environments: {}",
                mismatched_environments.join(", ")
            )
        };
        return Err(bad_request(message));
    }

    let crate_ids = repo_configs
        .iter()
        .map(|config| config.crate_id)
        .collect::<Vec<_>>();

    let new_token = AccessToken::generate();

    let trustpub_data = TrustpubData::GitHub {
        repository: signed_claims.repository,
        run_id: signed_claims.run_id,
        sha: signed_claims.sha,
    };

    let new_token_model = NewToken {
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
        hashed_token: &new_token.sha256(),
        crate_ids: &crate_ids,
        trustpub_data: Some(&trustpub_data),
    };

    new_token_model.insert(conn).await?;

    let token = new_token.finalize().expose_secret().into();
    Ok(Json(json::ExchangeResponse { token }))
}

async fn handle_gitlab_token(
    state: &AppState,
    unverified_jwt: &str,
    key: &DecodingKey,
) -> AppResult<Json<json::ExchangeResponse>> {
    let audience = &state.config.trustpub_audience;
    let signed_claims = GitLabClaims::decode(unverified_jwt, audience, key).map_err(|err| {
        warn!("Failed to decode JWT: {err}");
        bad_request("Failed to decode JWT")
    })?;

    let mut conn = state.db_write().await?;

    conn.transaction(|conn| Box::pin(handle_gitlab_token_inner(conn, signed_claims)))
        .await
}

async fn handle_gitlab_token_inner(
    conn: &mut AsyncPgConnection,
    signed_claims: GitLabClaims,
) -> AppResult<Json<json::ExchangeResponse>> {
    insert_jti(conn, &signed_claims.jti, signed_claims.exp).await?;

    // GitLab project paths can contain subgroups, which should be treated as
    // part of the namespace. We use `rsplit_once()` here to split after the
    // last slash in the full project path.
    //
    // In other words: `foo/bar/baz` becomes `(foo/bar, baz)`.
    let project_path = &signed_claims.project_path;
    let Some((namespace, project)) = project_path.rsplit_once('/') else {
        warn!("Unexpected project_path format in JWT: {project_path}");
        let message = "Unexpected `project_path` value";
        return Err(bad_request(message));
    };

    let Some(workflow_filepath) = signed_claims.workflow_filepath() else {
        let ci_config_ref_uri = &signed_claims.ci_config_ref_uri;
        warn!("Unexpected `ci_config_ref_uri` format in JWT: {ci_config_ref_uri}");
        let message = "Unexpected `ci_config_ref_uri` value";
        return Err(bad_request(message));
    };

    let mut repo_configs = GitLabConfig::query()
        .filter(lower(trustpub_configs_gitlab::namespace).eq(lower(&namespace)))
        .filter(lower(trustpub_configs_gitlab::project).eq(lower(&project)))
        .load(conn)
        .await?;

    if repo_configs.is_empty() {
        let message =
            format!("No Trusted Publishing config found for repository `{project_path}`.");
        return Err(bad_request(message));
    }

    // First, handle resurrection protection by lazily storing namespace_id and
    // verifying it on subsequent exchanges, before checking workflow/environment.
    let configs_to_update: Vec<i32> = repo_configs
        .iter()
        .filter(|config| config.namespace_id.is_none())
        .map(|config| config.id)
        .collect();

    if !configs_to_update.is_empty() {
        diesel::update(trustpub_configs_gitlab::table)
            .filter(trustpub_configs_gitlab::id.eq_any(&configs_to_update))
            .filter(trustpub_configs_gitlab::namespace_id.is_null())
            .set(trustpub_configs_gitlab::namespace_id.eq(&signed_claims.namespace_id))
            .execute(conn)
            .await?;
    }

    // Remove configs that have a stored namespace_id which doesn't match
    let mismatched_namespace_ids: Vec<String> = repo_configs
        .extract_if(.., |config| {
            config
                .namespace_id
                .as_ref()
                .is_some_and(|stored| stored != &signed_claims.namespace_id)
        })
        .filter_map(|config| config.namespace_id)
        .collect();

    if repo_configs.is_empty() {
        let message = format!(
            "The Trusted Publishing config for repository `{project_path}` does not match the namespace ID ({}) in the JWT. Expected namespace IDs: {}. Please recreate the Trusted Publishing config to update the namespace ID.",
            signed_claims.namespace_id,
            mismatched_namespace_ids.join(", ")
        );
        return Err(bad_request(message));
    }

    // Filter by workflow filepath match
    let mismatched_workflows: Vec<String> = repo_configs
        .extract_if(.., |config| config.workflow_filepath != workflow_filepath)
        .map(|config| format!("`{}`", config.workflow_filepath))
        .collect();

    if repo_configs.is_empty() {
        let message = format!(
            "The Trusted Publishing config for repository `{project_path}` does not match the workflow filepath `{workflow_filepath}` in the JWT. Expected workflow filepaths: {}",
            mismatched_workflows.join(", ")
        );
        return Err(bad_request(message));
    }

    // Filter by environment (if config requires one)
    let mismatched_environments: Vec<String> = repo_configs
        .extract_if(.., |config| {
            match (&config.environment, &signed_claims.environment) {
                // Keep configs with no environment requirement
                (None, _) => false,
                // Remove configs requiring environment when JWT has none
                (Some(_), None) => true,
                // Remove non-matching environments (case-insensitive)
                (Some(config_env), Some(signed_env)) => {
                    config_env.to_lowercase() != signed_env.to_lowercase()
                }
            }
        })
        .filter_map(|config| config.environment.map(|env| format!("`{env}`")))
        .collect();

    if repo_configs.is_empty() {
        let message = if let Some(signed_environment) = &signed_claims.environment {
            format!(
                "The Trusted Publishing config for repository `{project_path}` does not match the environment `{signed_environment}` in the JWT. Expected environments: {}",
                mismatched_environments.join(", ")
            )
        } else {
            format!(
                "The Trusted Publishing config for repository `{project_path}` requires an environment, but the JWT does not specify one. Expected environments: {}",
                mismatched_environments.join(", ")
            )
        };
        return Err(bad_request(message));
    }

    let crate_ids = repo_configs
        .iter()
        .map(|config| config.crate_id)
        .collect::<Vec<_>>();

    let new_token = AccessToken::generate();

    let trustpub_data = TrustpubData::GitLab {
        project_path: signed_claims.project_path,
        job_id: signed_claims.job_id,
        sha: signed_claims.sha,
    };

    let new_token_model = NewToken {
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
        hashed_token: &new_token.sha256(),
        crate_ids: &crate_ids,
        trustpub_data: Some(&trustpub_data),
    };

    new_token_model.insert(conn).await?;

    let token = new_token.finalize().expose_secret().into();
    Ok(Json(json::ExchangeResponse { token }))
}
