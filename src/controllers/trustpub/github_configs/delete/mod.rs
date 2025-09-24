use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::email::EmailMessage;
use crate::util::errors::{AppResult, bad_request, not_found};
use anyhow::Context;
use axum::extract::Path;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::trustpub::GitHubConfig;
use crates_io_database::models::{Crate, OwnerKind};
use crates_io_database::schema::{crate_owners, crates, emails, trustpub_configs_github, users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use http::request::Parts;
use minijinja::context;
use tracing::warn;

#[cfg(test)]
mod tests;

/// Delete Trusted Publishing configuration for GitHub Actions.
#[utoipa::path(
    delete,
    path = "/api/v1/trusted_publishing/github_configs/{id}",
    params(
        ("id" = i32, Path, description = "ID of the Trusted Publishing configuration"),
    ),
    security(("cookie" = []), ("api_token" = [])),
    tag = "trusted_publishing",
    responses((status = 204, description = "Successful Response")),
)]
pub async fn delete_trustpub_github_config(
    state: AppState,
    Path(id): Path<i32>,
    parts: Parts,
) -> AppResult<StatusCode> {
    let mut conn = state.db_write().await?;

    // First, find the config and crate to get the crate name for scope validation
    let (config, krate) = trustpub_configs_github::table
        .inner_join(crates::table)
        .filter(trustpub_configs_github::id.eq(id))
        .select((GitHubConfig::as_select(), Crate::as_select()))
        .first::<(GitHubConfig, Crate)>(&mut conn)
        .await
        .optional()?
        .ok_or_else(not_found)?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(&krate.name)
        .check(&parts, &mut conn)
        .await?;
    let auth_user = auth.user();

    // Load all crate owners for the given crate ID
    let user_owners = crate_owners::table
        .filter(crate_owners::crate_id.eq(config.crate_id))
        .filter(crate_owners::deleted.eq(false))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .inner_join(users::table)
        .inner_join(emails::table.on(users::id.eq(emails::user_id)))
        .select((users::id, users::gh_login, emails::email, emails::verified))
        .load::<(i32, String, String, bool)>(&mut conn)
        .await?;

    // Check if the authenticated user is an owner of the crate
    if !user_owners.iter().any(|owner| owner.0 == auth_user.id) {
        return Err(bad_request("You are not an owner of this crate"));
    }

    // Delete the configuration from the database
    diesel::delete(trustpub_configs_github::table.filter(trustpub_configs_github::id.eq(id)))
        .execute(&mut conn)
        .await?;

    // Send notification emails to crate owners

    let recipients = user_owners
        .into_iter()
        .filter(|(_, _, _, verified)| *verified)
        .map(|(_, login, email, _)| (login, email))
        .collect::<Vec<_>>();

    for (recipient, email_address) in &recipients {
        let context = context! { recipient, auth_user, krate, config };

        if let Err(err) = send_notification_email(&state, email_address, context).await {
            warn!("Failed to send trusted publishing notification to {email_address}: {err}");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn send_notification_email(
    state: &AppState,
    email_address: &str,
    context: minijinja::Value,
) -> anyhow::Result<()> {
    let email = EmailMessage::from_template("trustpub_config_deleted", context)
        .context("Failed to render email template")?;

    state
        .emails
        .send(email_address, email)
        .await
        .context("Failed to send email")
}
