use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::krate::CratePath;
use crate::email::EmailMessage;
use crate::middleware::real_ip::RealIp;
use crate::models::token::EndpointScope;
use crate::models::{Crate, User};
use crate::schema::*;
use crate::util::errors::{AppResult, crate_not_found, custom};
use crate::views::EncodableCrate;
use anyhow::Context;
use axum::{Extension, Json};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use http::{StatusCode, request::Parts};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PatchRequest {
    /// The crate settings to update.
    #[serde(rename = "crate")]
    #[schema(inline)]
    pub krate: PatchRequestCrate,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PatchRequestCrate {
    /// Whether this crate can only be published via Trusted Publishing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trustpub_only: Option<bool>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PatchResponse {
    /// The updated crate metadata.
    #[serde(rename = "crate")]
    krate: EncodableCrate,
}

/// Update crate settings.
#[utoipa::path(
    patch,
    path = "/api/v1/crates/{name}",
    params(CratePath),
    request_body = inline(PatchRequest),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "crates",
    responses((status = 200, description = "Successful Response", body = inline(PatchResponse))),
)]
pub async fn update_crate(
    app: AppState,
    path: CratePath,
    req: Parts,
    Extension(real_ip): Extension<RealIp>,
    Json(body): Json<PatchRequest>,
) -> AppResult<Json<PatchResponse>> {
    let mut conn = app.db_write().await?;

    // Check that the crate exists
    let krate = path.load_crate(&mut conn).await?;

    // Check that the user is authenticated with appropriate permissions
    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::TrustedPublishing)
        .for_crate(&krate.name)
        .check(&req, &mut conn)
        .await?;

    auth.reject_legacy_tokens()?;

    // Update crate settings in a transaction
    conn.transaction(|conn| {
        update_inner(conn, &app, &krate, auth.user(), &real_ip, body).scope_boxed()
    })
    .await
}

async fn update_inner(
    conn: &mut diesel_async::AsyncPgConnection,
    app: &AppState,
    krate: &Crate,
    user: &User,
    real_ip: &RealIp,
    body: PatchRequest,
) -> AppResult<Json<PatchResponse>> {
    // Query user owners to check permissions and send emails
    let user_owners = crate_owners::table
        .inner_join(users::table)
        .inner_join(emails::table.on(users::id.eq(emails::user_id)))
        .filter(crate_owners::crate_id.eq(krate.id))
        .filter(crate_owners::deleted.eq(false))
        .filter(crate_owners::owner_kind.eq(crate::models::OwnerKind::User))
        .select((users::id, users::gh_login, emails::email, emails::verified))
        .load::<(i32, String, String, bool)>(conn)
        .await?;

    // Check that the authenticated user is an owner
    if !user_owners.iter().any(|(id, _, _, _)| *id == user.id) {
        let msg = "only owners have permission to modify crate settings";
        return Err(custom(StatusCode::FORBIDDEN, msg));
    }

    // Update trustpub_only if provided
    if let Some(trustpub_only) = body.krate.trustpub_only
        && trustpub_only != krate.trustpub_only
    {
        diesel::update(crates::table)
            .filter(crates::id.eq(krate.id))
            .set(crates::trustpub_only.eq(trustpub_only))
            .execute(conn)
            .await?;

        // Audit log the setting change
        info!(
            target: "audit",
            action = "trustpub_only_change",
            krate.name = %krate.name,
            network.client.ip = %**real_ip,
            usr.id = user.id,
            usr.name = %user.gh_login,
            "User {} set trustpub_only={trustpub_only} for crate {}",
            user.gh_login,
            krate.name
        );

        // Send email notifications to all crate owners
        for (_, gh_login, email_address, email_verified) in &user_owners {
            if *email_verified {
                let email = TrustpubOnlyChangedEmail {
                    recipient: gh_login,
                    auth_user: user,
                    krate,
                    trustpub_only,
                };

                if let Err(err) = email.send(app, email_address).await {
                    warn!("Failed to send trustpub_only notification to {email_address}: {err}");
                }
            }
        }
    }

    // Reload the crate to get updated data
    let (krate, downloads, recent_downloads, default_version, yanked, num_versions): (
        Crate,
        i64,
        Option<i64>,
        Option<String>,
        Option<bool>,
        Option<i32>,
    ) = Crate::by_name(&krate.name)
        .inner_join(crate_downloads::table)
        .left_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .select((
            Crate::as_select(),
            crate_downloads::downloads,
            recent_crate_downloads::downloads.nullable(),
            versions::num.nullable(),
            versions::yanked.nullable(),
            default_versions::num_versions.nullable(),
        ))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&krate.name))?;

    let encodable_crate = EncodableCrate::from(
        krate,
        default_version.as_deref(),
        num_versions.unwrap_or_default(),
        yanked,
        None,
        None,
        None,
        None,
        false,
        downloads,
        recent_downloads,
    );

    Ok(Json(PatchResponse {
        krate: encodable_crate,
    }))
}

#[derive(Serialize)]
struct TrustpubOnlyChangedEmail<'a> {
    /// The GitHub login of the email recipient.
    recipient: &'a str,
    /// The user who changed the setting.
    auth_user: &'a User,
    /// The crate for which the setting was changed.
    krate: &'a Crate,
    /// The new value of the trustpub_only flag.
    trustpub_only: bool,
}

impl TrustpubOnlyChangedEmail<'_> {
    async fn send(&self, state: &AppState, email_address: &str) -> anyhow::Result<()> {
        let email = EmailMessage::from_template("trustpub_only_changed", self);
        let email = email.context("Failed to render email template")?;

        state
            .emails
            .send(email_address, email)
            .await
            .context("Failed to send email")
    }
}
