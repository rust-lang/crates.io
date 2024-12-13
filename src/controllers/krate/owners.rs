//! All routes related to managing owners of a crate

use crate::models::{krate::NewOwnerInvite, token::EndpointScope};
use crate::models::{Crate, Owner, Rights, Team, User};
use crate::util::errors::{bad_request, crate_not_found, custom, AppResult};
use crate::views::EncodableOwner;
use crate::{app::AppState, models::krate::OwnerAddError};
use crate::{auth::AuthCheck, email::Email};
use axum::extract::Path;
use axum::Json;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use http::request::Parts;
use http::StatusCode;
use secrecy::{ExposeSecret, SecretString};

/// List crate owners.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owners",
    operation_id = "list_owners",
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn owners(state: AppState, Path(crate_name): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let krate: Crate = Crate::by_name(&crate_name)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&crate_name))?;

    let owners = krate
        .owners(&mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(json!({ "users": owners }))
}

/// List team owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_team",
    operation_id = "get_team_owners",
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn owner_team(state: AppState, Path(crate_name): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;
    let krate: Crate = Crate::by_name(&crate_name)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&crate_name))?;

    let owners = Team::owning(&krate, &mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(json!({ "teams": owners }))
}

/// Handles the `GET /crates/:crate_id/owner_user` route.
pub async fn owner_user(state: AppState, Path(crate_name): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let krate: Crate = Crate::by_name(&crate_name)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&crate_name))?;

    let owners = User::owning(&krate, &mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(json!({ "users": owners }))
}

/// Add crate owners.
#[utoipa::path(
    put,
    path = "/api/v1/crates/{name}/owners",
    operation_id = "add_owners",
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn add_owners(
    app: AppState,
    Path(crate_name): Path<String>,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<ErasedJson> {
    modify_owners(app, crate_name, parts, body, true).await
}

/// Remove crate owners.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/owners",
    operation_id = "delete_owners",
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn remove_owners(
    app: AppState,
    Path(crate_name): Path<String>,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<ErasedJson> {
    modify_owners(app, crate_name, parts, body, false).await
}

#[derive(Deserialize)]
pub struct ChangeOwnersRequest {
    #[serde(alias = "users")]
    owners: Vec<String>,
}

async fn modify_owners(
    app: AppState,
    crate_name: String,
    parts: Parts,
    body: ChangeOwnersRequest,
    add: bool,
) -> AppResult<ErasedJson> {
    let logins = body.owners;

    // Bound the number of invites processed per request to limit the cost of
    // processing them all.
    if logins.len() > 10 {
        return Err(bad_request(
            "too many invites for this request - maximum 10",
        ));
    }

    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::ChangeOwners)
        .for_crate(&crate_name)
        .check(&parts, &mut conn)
        .await?;

    let user = auth.user();

    let (comma_sep_msg, emails) = conn
        .transaction(|conn| {
            let app = app.clone();
            async move {
                let krate: Crate = Crate::by_name(&crate_name)
                    .first(conn)
                    .await
                    .optional()?
                    .ok_or_else(|| crate_not_found(&crate_name))?;

                let owners = krate.owners(conn).await?;

                match user.rights(&app, &owners).await? {
                    Rights::Full => {}
                    // Yes!
                    Rights::Publish => {
                        return Err(custom(
                            StatusCode::FORBIDDEN,
                            "team members don't have permission to modify owners",
                        ));
                    }
                    Rights::None => {
                        return Err(custom(
                            StatusCode::FORBIDDEN,
                            "only owners have permission to modify owners",
                        ));
                    }
                }

                // The set of emails to send out after invite processing is complete and
                // the database transaction has committed.
                let mut emails = Vec::with_capacity(logins.len());

                let comma_sep_msg = if add {
                    let mut msgs = Vec::with_capacity(logins.len());
                    for login in &logins {
                        let login_test =
                            |owner: &Owner| owner.login().to_lowercase() == *login.to_lowercase();
                        if owners.iter().any(login_test) {
                            return Err(bad_request(format_args!("`{login}` is already an owner")));
                        }

                        match krate.owner_add(&app, conn, user, login).await {
                            // A user was successfully invited, and they must accept
                            // the invite, and a best-effort attempt should be made
                            // to email them the invite token for one-click
                            // acceptance.
                            Ok(NewOwnerInvite::User(invitee, token)) => {
                                msgs.push(format!(
                                    "user {} has been invited to be an owner of crate {}",
                                    invitee.gh_login, krate.name,
                                ));

                                if let Some(recipient) =
                                    invitee.verified_email(conn).await.ok().flatten()
                                {
                                    emails.push(OwnerInviteEmail {
                                        recipient_email_address: recipient,
                                        inviter: user.gh_login.clone(),
                                        domain: app.emails.domain.clone(),
                                        crate_name: krate.name.clone(),
                                        token,
                                    });
                                }
                            }

                            // A team was successfully invited. They are immediately
                            // added, and do not have an invite token.
                            Ok(NewOwnerInvite::Team(team)) => msgs.push(format!(
                                "team {} has been added as an owner of crate {}",
                                team.login, krate.name
                            )),

                            // This user has a pending invite.
                            Err(OwnerAddError::AlreadyInvited(user)) => msgs.push(format!(
                            "user {} already has a pending invitation to be an owner of crate {}",
                            user.gh_login, krate.name
                        )),

                            // An opaque error occurred.
                            Err(OwnerAddError::AppError(e)) => return Err(e),
                        }
                    }
                    msgs.join(",")
                } else {
                    for login in &logins {
                        krate.owner_remove(conn, login).await?;
                    }
                    if User::owning(&krate, conn).await?.is_empty() {
                        return Err(bad_request(
                            "cannot remove all individual owners of a crate. \
                     Team member don't have permission to modify owners, so \
                     at least one individual owner is required.",
                        ));
                    }
                    "owners successfully removed".to_owned()
                };

                Ok((comma_sep_msg, emails))
            }
            .scope_boxed()
        })
        .await?;

    // Send the accumulated invite emails now the database state has
    // committed.
    for email in emails {
        let addr = email.recipient_email_address().to_string();

        if let Err(e) = app.emails.send(&addr, email).await {
            warn!("Failed to send co-owner invite email: {e}");
        }
    }

    Ok(json!({ "msg": comma_sep_msg, "ok": true }))
}

pub struct OwnerInviteEmail {
    /// The destination email address for this email.
    recipient_email_address: String,

    /// Email body variables.
    inviter: String,
    domain: String,
    crate_name: String,
    token: SecretString,
}

impl OwnerInviteEmail {
    pub fn recipient_email_address(&self) -> &str {
        &self.recipient_email_address
    }
}

impl Email for OwnerInviteEmail {
    fn subject(&self) -> String {
        format!(
            "crates.io: Ownership invitation for \"{}\"",
            self.crate_name
        )
    }

    fn body(&self) -> String {
        format!(
            "{user_name} has invited you to become an owner of the crate {crate_name}!\n
Visit https://{domain}/accept-invite/{token} to accept this invitation,
or go to https://{domain}/me/pending-invites to manage all of your crate ownership invitations.",
            user_name = self.inviter,
            domain = self.domain,
            crate_name = self.crate_name,
            token = self.token.expose_secret(),
        )
    }
}
