use super::json;
use crate::app::AppState;
use crate::util::errors::{AppResult, bad_request, server_error};
use axum::Json;
use crates_io_database::models::trustpub::{NewToken, NewUsedJti, TrustpubData};
use crates_io_database::schema::trustpub_configs_github;
use crates_io_diesel_helpers::lower;
use crates_io_trustpub::access_token::AccessToken;
use crates_io_trustpub::github::{GITHUB_ISSUER_URL, GitHubClaims};
use crates_io_trustpub::unverified::UnverifiedClaims;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use secrecy::ExposeSecret;
use tracing::warn;

#[cfg(test)]
mod tests;

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
        let error = format!("Unsupported JWT issuer: {unverified_issuer}");
        return Err(bad_request(error));
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

    // The following code is only supporting GitHub Actions for now, so let's
    // drop out if the issuer is not GitHub.
    if unverified_issuer != GITHUB_ISSUER_URL {
        let error = format!("Unsupported JWT issuer: {unverified_issuer}");
        return Err(bad_request(error));
    }

    let audience = &state.config.trustpub_audience;
    let signed_claims = GitHubClaims::decode(&unverified_jwt, audience, &key).map_err(|err| {
        warn!("Failed to decode JWT: {err}");
        bad_request("Failed to decode JWT")
    })?;

    let mut conn = state.db_write().await?;

    conn.transaction(|conn| {
        async move {
            let used_jti = NewUsedJti::new(&signed_claims.jti, signed_claims.exp);
            match used_jti.insert(conn).await {
                Ok(_) => {} // JTI was successfully inserted, continue
                Err(DatabaseError(UniqueViolation, _)) => {
                    warn!("Attempted JWT reuse (jti: {})", signed_claims.jti);
                    let detail = "JWT has already been used";
                    return Err(bad_request(detail));
                }
                Err(err) => Err(err)?,
            };

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

            let crate_ids = trustpub_configs_github::table
                .select(trustpub_configs_github::crate_id)
                .filter(trustpub_configs_github::repository_owner_id.eq(&repository_owner_id))
                .filter(
                    lower(trustpub_configs_github::repository_owner).eq(lower(&repository_owner)),
                )
                .filter(lower(trustpub_configs_github::repository_name).eq(lower(&repository_name)))
                .filter(trustpub_configs_github::workflow_filename.eq(&workflow_filename))
                .filter(
                    trustpub_configs_github::environment
                        .is_null()
                        .or(lower(trustpub_configs_github::environment)
                            .eq(lower(&signed_claims.environment))),
                )
                .load::<i32>(conn)
                .await?;

            if crate_ids.is_empty() {
                warn!("No matching Trusted Publishing config found");
                let message = "No matching Trusted Publishing config found";
                return Err(bad_request(message));
            }

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
        .scope_boxed()
    })
    .await
}
