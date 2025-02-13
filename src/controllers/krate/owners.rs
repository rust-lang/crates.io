//! All routes related to managing owners of a crate

use crate::controllers::krate::CratePath;
use crate::models::krate::OwnerRemoveError;
use crate::models::{
    krate::NewOwnerInvite, token::EndpointScope, CrateOwner, NewCrateOwnerInvitation,
    NewCrateOwnerInvitationOutcome, OwnerKind,
};
use crate::models::{Crate, Owner, Rights, Team, User};
use crate::util::errors::{bad_request, crate_not_found, custom, AppResult, BoxedAppError};
use crate::views::EncodableOwner;
use crate::{app::AppState, App};
use crate::{auth::AuthCheck, email::Email};
use axum::Json;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use chrono::Utc;
use crates_io_database::schema::crate_owners;
use crates_io_github::GitHubClient;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use http::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

/// List crate owners.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn list_owners(state: AppState, path: CratePath) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let krate = path.load_crate(&mut conn).await?;

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
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get_team_owners(state: AppState, path: CratePath) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;
    let krate = path.load_crate(&mut conn).await?;

    let owners = Team::owning(&krate, &mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(json!({ "teams": owners }))
}

/// List user owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_user",
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get_user_owners(state: AppState, path: CratePath) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let krate = path.load_crate(&mut conn).await?;

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
    params(CratePath),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn add_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<ErasedJson> {
    modify_owners(app, path.name, parts, body, true).await
}

/// Remove crate owners.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn remove_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<ErasedJson> {
    modify_owners(app, path.name, parts, body, false).await
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

                match user.rights(&*app.github, &owners).await? {
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

                        match add_owner(&app, conn, user, &krate, login).await {
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
                            Err(OwnerAddError::Diesel(e)) => return Err(e.into()),
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

/// Invite `login` as an owner of this crate, returning the created
/// [`NewOwnerInvite`].
async fn add_owner(
    app: &App,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    login: &str,
) -> Result<NewOwnerInvite, OwnerAddError> {
    if login.contains(':') {
        add_team_owner(&*app.github, conn, req_user, krate, login).await
    } else {
        invite_user_owner(app, conn, req_user, krate, login).await
    }
}

async fn invite_user_owner(
    app: &App,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    login: &str,
) -> Result<NewOwnerInvite, OwnerAddError> {
    let user = User::find_by_login(conn, login)
        .await
        .optional()?
        .ok_or_else(|| bad_request(format_args!("could not find user with login `{login}`")))?;

    // Users are invited and must accept before being added
    let expires_at = Utc::now() + app.config.ownership_invitations_expiration;
    let invite = NewCrateOwnerInvitation {
        invited_user_id: user.id,
        invited_by_user_id: req_user.id,
        crate_id: krate.id,
        expires_at,
    };

    match invite.create(conn).await? {
        NewCrateOwnerInvitationOutcome::InviteCreated { plaintext_token } => {
            Ok(NewOwnerInvite::User(user, plaintext_token))
        }
        NewCrateOwnerInvitationOutcome::AlreadyExists => {
            Err(OwnerAddError::AlreadyInvited(Box::new(user)))
        }
    }
}

async fn add_team_owner(
    gh_client: &dyn GitHubClient,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    login: &str,
) -> Result<NewOwnerInvite, OwnerAddError> {
    // Always recreate teams to get the most up-to-date GitHub ID
    let team = Team::create_or_update(gh_client, conn, login, req_user).await?;

    // Teams are added as owners immediately, since the above call ensures
    // the user is a team member.
    diesel::insert_into(crate_owners::table)
        .values(&CrateOwner {
            crate_id: krate.id,
            owner_id: team.id,
            created_by: req_user.id,
            owner_kind: OwnerKind::Team,
            email_notifications: true,
        })
        .on_conflict(crate_owners::table.primary_key())
        .do_update()
        .set(crate_owners::deleted.eq(false))
        .execute(conn)
        .await?;

    Ok(NewOwnerInvite::Team(team))
}

/// Error results from a [`add_owner()`] model call.
#[derive(Debug, Error)]
enum OwnerAddError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
    /// An opaque [`BoxedAppError`].
    #[error("{0}")] // AppError does not impl Error
    AppError(BoxedAppError),

    /// The requested invitee already has a pending invite.
    ///
    /// Note: Teams are always immediately added, so they cannot have a pending
    /// invite to cause this error.
    #[error("user already has pending invite")]
    AlreadyInvited(Box<User>),
}

/// A [`BoxedAppError`] does not impl [`std::error::Error`] so it needs a manual
/// [`From`] impl.
impl From<BoxedAppError> for OwnerAddError {
    fn from(value: BoxedAppError) -> Self {
        Self::AppError(value)
    }
}

impl From<OwnerRemoveError> for BoxedAppError {
    fn from(error: OwnerRemoveError) -> Self {
        match error {
            OwnerRemoveError::Diesel(error) => error.into(),
            OwnerRemoveError::NotFound { login } => {
                bad_request(format!("could not find owner with login `{login}`"))
            }
        }
    }
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
