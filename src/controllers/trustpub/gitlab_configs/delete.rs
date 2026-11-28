use crate::auth::AuthCheck;
use crate::controllers::trustpub::emails::{ConfigDeletedEmail, ConfigType};
use crate::server::ServerContext;
use crate::util::errors::{AppResult, bad_request, forbidden, not_found};
use anyhow::Context;
use axum::extract::Path;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::trustpub::GitLabConfig;
use crates_io_database::models::{Crate, OwnerKind};
use crates_io_database::schema::{crate_owners, crates, emails, trustpub_configs_gitlab, users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use http::request::Parts;
use tracing::warn;

/// Delete Trusted Publishing configuration for GitLab CI/CD.
#[utoipa::path(
    delete,
    path = "/api/v1/trusted_publishing/gitlab_configs/{id}",
    params(
        ("id" = i32, Path, description = "ID of the Trusted Publishing configuration"),
    ),
    security(("cookie" = []), ("api_token" = [])),
    tag = "trusted_publishing",
    responses(
        (status = 204, description = "Successful Response"),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn delete_trustpub_gitlab_config(
    ctx: ServerContext,
    Path(id): Path<i32>,
    parts: Parts,
) -> AppResult<StatusCode> {
    let mut conn = ctx.db_write().await?;

    // First, find the config and crate to get the crate name for scope validation
    let (config, krate) = trustpub_configs_gitlab::table
        .inner_join(crates::table)
        .filter(trustpub_configs_gitlab::id.eq(id))
        .select((GitLabConfig::as_select(), Crate::as_select()))
        .first::<(GitLabConfig, Crate)>(&mut conn)
        .await
        .optional()?
        .ok_or_else(not_found)?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(&krate.name)
        .reject_legacy_tokens()
        .check(&parts, &mut conn)
        .await?;
    let auth_user = auth.user();

    // Load all crate owners for the given crate ID
    let user_owners = crate_owners::table
        .filter(crate_owners::crate_id.eq(config.crate_id))
        .filter(crate_owners::deleted.eq(false))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .inner_join(users::table)
        .left_join(emails::table.on(users::id.eq(emails::user_id).and(emails::verified.eq(true))))
        .select((users::id, users::username, emails::email.nullable()))
        .load::<(i32, String, Option<String>)>(&mut conn)
        .await?;

    // Check if the authenticated user is an owner of the crate
    let (_, _, verified_email) = user_owners
        .iter()
        .find(|(id, _, _)| *id == auth_user.id)
        .ok_or_else(|| bad_request("You are not an owner of this crate"))?;

    if verified_email.is_none() {
        let message = "You must verify your email address to delete a Trusted Publishing config";
        return Err(forbidden(message));
    }

    // Delete the configuration from the database
    diesel::delete(trustpub_configs_gitlab::table.filter(trustpub_configs_gitlab::id.eq(id)))
        .execute(&mut conn)
        .await?;

    // Send notification emails to crate owners

    let recipients = user_owners
        .into_iter()
        .filter_map(|(_, login, email)| Some((login, email?)))
        .collect::<Vec<_>>();

    for (recipient, email_address) in &recipients {
        let config = ConfigType::GitLab(&config);

        let context = ConfigDeletedEmail {
            recipient,
            auth_user,
            krate: &krate,
            config,
        };

        if let Err(err) = send_notification_email(&ctx, email_address, context).await {
            warn!("Failed to send trusted publishing notification to {email_address}: {err:#}");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn send_notification_email(
    ctx: &ServerContext,
    email_address: &str,
    context: ConfigDeletedEmail<'_>,
) -> anyhow::Result<()> {
    let email = context.render();
    let email = email.context("Failed to render email template")?;

    ctx.emails
        .send(email_address, email)
        .await
        .context("Failed to send email")
}
