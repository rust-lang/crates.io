use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::krate::load_crate;
use crate::controllers::trustpub::emails::{ConfigCreatedEmail, ConfigType};
use crate::controllers::trustpub::gitlab_configs::json;
use crate::util::errors::{AppResult, bad_request, custom, forbidden};
use anyhow::Context;
use axum::Json;
use crates_io_database::models::OwnerKind;
use crates_io_database::models::token::EndpointScope;
use crates_io_database::models::trustpub::{GitLabConfig, NewGitLabConfig};
use crates_io_database::schema::{crate_owners, emails, users};
use crates_io_trustpub::gitlab::validation::{
    validate_environment, validate_namespace, validate_project, validate_workflow_filepath,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;
use tracing::warn;

const MAX_CONFIGS_PER_CRATE: usize = 5;

#[utoipa::path(
    post,
    path = "/api/v1/trusted_publishing/gitlab_configs",
    security(("cookie" = []), ("api_token" = [])),
    request_body = inline(json::CreateRequest),
    tag = "trusted_publishing",
    responses((status = 200, description = "Successful Response", body = inline(json::CreateResponse))),
)]
pub async fn create_trustpub_gitlab_config(
    state: AppState,
    parts: Parts,
    json: json::CreateRequest,
) -> AppResult<Json<json::CreateResponse>> {
    let json_config = json.gitlab_config;

    validate_namespace(&json_config.namespace)?;
    validate_project(&json_config.project)?;
    validate_workflow_filepath(&json_config.workflow_filepath)?;
    if let Some(env) = &json_config.environment {
        validate_environment(env)?;
    }

    let mut conn = state.db_write().await?;

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(&json_config.krate)
        .check(&parts, &mut conn)
        .await?;
    let auth_user = auth.user();

    let krate = load_crate(&mut conn, &json_config.krate).await?;

    // Check if the crate has reached the maximum number of configs
    let config_count = GitLabConfig::count_for_crate(&mut conn, krate.id).await?;
    if config_count >= MAX_CONFIGS_PER_CRATE as i64 {
        let message = format!(
            "This crate already has the maximum number of GitLab Trusted Publishing configurations ({})",
            MAX_CONFIGS_PER_CRATE
        );
        return Err(custom(http::StatusCode::CONFLICT, message));
    }

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

    // Save the new GitLab OIDC config to the database

    let new_config = NewGitLabConfig {
        crate_id: krate.id,
        namespace: &json_config.namespace,
        project: &json_config.project,
        workflow_filepath: &json_config.workflow_filepath,
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
        let saved_config = ConfigType::GitLab(&saved_config);

        let context = ConfigCreatedEmail {
            recipient,
            auth_user,
            krate: &krate,
            saved_config,
        };

        if let Err(err) = send_notification_email(&state, email_address, context).await {
            warn!("Failed to send trusted publishing notification to {email_address}: {err}");
        }
    }

    let gitlab_config = json::GitLabConfig {
        id: saved_config.id,
        krate: krate.name,
        namespace: saved_config.namespace,
        namespace_id: saved_config.namespace_id,
        project: saved_config.project,
        workflow_filepath: saved_config.workflow_filepath,
        environment: saved_config.environment,
        created_at: saved_config.created_at,
    };

    Ok(Json(json::CreateResponse { gitlab_config }))
}

async fn send_notification_email(
    state: &AppState,
    email_address: &str,
    context: ConfigCreatedEmail<'_>,
) -> anyhow::Result<()> {
    let email = context.render();
    let email = email.context("Failed to render email template")?;

    state
        .emails
        .send(email_address, email)
        .await
        .context("Failed to send email")
}
