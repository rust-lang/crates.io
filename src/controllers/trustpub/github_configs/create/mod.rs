use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::krate::load_crate;
use crate::controllers::trustpub::github_configs::json;
use crate::email::EmailMessage;
use crate::util::errors::{AppResult, bad_request, forbidden};
use anyhow::Context;
use axum::Json;
use crates_io_database::models::OwnerKind;
use crates_io_database::models::trustpub::NewGitHubConfig;
use crates_io_database::schema::{crate_owners, emails, users};
use crates_io_github::GitHubError;
use crates_io_trustpub::github::validation::{
    validate_environment, validate_owner, validate_repo, validate_workflow_filename,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use minijinja::context;
use oauth2::AccessToken;
use secrecy::ExposeSecret;
use tracing::warn;

#[cfg(test)]
mod tests;

/// Create a new Trusted Publishing configuration for GitHub Actions.
#[utoipa::path(
    post,
    path = "/api/v1/trusted_publishing/github_configs",
    security(("cookie" = [])),
    request_body = inline(json::CreateRequest),
    tag = "trusted_publishing",
    responses((status = 200, description = "Successful Response", body = inline(json::CreateResponse))),
)]
pub async fn create_trustpub_github_config(
    state: AppState,
    parts: Parts,
    json: json::CreateRequest,
) -> AppResult<Json<json::CreateResponse>> {
    let json_config = json.github_config;

    validate_owner(&json_config.repository_owner)?;
    validate_repo(&json_config.repository_name)?;
    validate_workflow_filename(&json_config.workflow_filename)?;
    if let Some(env) = &json_config.environment {
        validate_environment(env)?;
    }

    let mut conn = state.db_write().await?;

    let auth = AuthCheck::only_cookie().check(&parts, &mut conn).await?;
    let auth_user = auth.user();

    let krate = load_crate(&mut conn, &json_config.krate).await?;

    let user_owners = crate_owners::table
        .filter(crate_owners::crate_id.eq(krate.id))
        .filter(crate_owners::deleted.eq(false))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .inner_join(users::table)
        .inner_join(emails::table.on(users::id.eq(emails::user_id)))
        .select((users::id, users::gh_login, emails::email, emails::verified))
        .load::<(i32, String, String, bool)>(&mut conn)
        .await?;

    let (_, _, _, email_verified) = user_owners
        .iter()
        .find(|(id, _, _, _)| *id == auth_user.id)
        .ok_or_else(|| bad_request("You are not an owner of this crate"))?;

    if !email_verified {
        let message = "You must verify your email address to create a Trusted Publishing config";
        return Err(forbidden(message));
    }

    // Lookup `repository_owner_id` via GitHub API

    let owner = &json_config.repository_owner;
    let gh_auth = &auth_user.gh_access_token;
    let gh_auth = AccessToken::new(gh_auth.expose_secret().to_string());
    let github_user = match state.github.get_user(owner, &gh_auth).await {
        Ok(user) => user,
        Err(GitHubError::NotFound(_)) => Err(bad_request("Unknown GitHub user or organization"))?,
        Err(err) => Err(err)?,
    };

    // Save the new GitHub OIDC config to the database

    let new_config = NewGitHubConfig {
        crate_id: krate.id,
        // Use the normalized owner name as provided by GitHub.
        repository_owner: &github_user.login,
        repository_owner_id: github_user.id,
        repository_name: &json_config.repository_name,
        workflow_filename: &json_config.workflow_filename,
        environment: json_config.environment.as_deref(),
    };

    let saved_config = new_config.insert(&mut conn).await?;

    // Send notification emails to crate owners

    let recipients = user_owners
        .into_iter()
        .filter(|(_, _, _, verified)| *verified)
        .map(|(_, login, email, _)| (login, email))
        .collect::<Vec<_>>();

    for (recipient, email_address) in &recipients {
        let context = context! { recipient, auth_user, krate, saved_config };

        if let Err(err) = send_notification_email(&state, email_address, context).await {
            warn!("Failed to send trusted publishing notification to {email_address}: {err}");
        }
    }

    let github_config = json::GitHubConfig {
        id: saved_config.id,
        krate: krate.name,
        repository_owner: saved_config.repository_owner,
        repository_owner_id: saved_config.repository_owner_id,
        repository_name: saved_config.repository_name,
        workflow_filename: saved_config.workflow_filename,
        environment: saved_config.environment,
        created_at: saved_config.created_at,
    };

    Ok(Json(json::CreateResponse { github_config }))
}

async fn send_notification_email(
    state: &AppState,
    email_address: &str,
    context: minijinja::Value,
) -> anyhow::Result<()> {
    let email = EmailMessage::from_template("config_created", context)
        .context("Failed to render email template")?;

    state
        .emails
        .send(email_address, email)
        .await
        .context("Failed to send email")
}
