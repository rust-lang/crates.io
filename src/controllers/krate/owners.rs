//! All routes related to managing owners of a crate

use crate::controllers::helpers::authorization::Rights;
use crate::controllers::krate::CratePath;
use crate::models::krate::OwnerRemoveError;
use crate::models::{Crate, Owner, Team, User};
use crate::models::{
    CrateOwner, NewCrateOwnerInvitation, NewCrateOwnerInvitationOutcome, NewTeam,
    krate::NewOwnerInvite, token::EndpointScope,
};
use crate::util::errors::{AppResult, BoxedAppError, bad_request, crate_not_found, custom};
use crate::util::gh_token_encryption::GitHubTokenEncryption;
use crate::views::EncodableOwner;
use crate::{App, app::AppState};
use crate::{auth::AuthCheck, email::EmailMessage};
use axum::Json;
use chrono::Utc;
use crates_io_github::{GitHubClient, GitHubError};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use http::request::Parts;
use minijinja::context;
use oauth2::AccessToken;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UsersResponse {
    pub users: Vec<EncodableOwner>,
}

/// List crate owners.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(UsersResponse))),
)]
pub async fn list_owners(state: AppState, path: CratePath) -> AppResult<Json<UsersResponse>> {
    let mut conn = state.db_read().await?;

    let krate = path.load_crate(&mut conn).await?;

    let users = krate
        .owners(&mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(Json(UsersResponse { users }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TeamsResponse {
    pub teams: Vec<EncodableOwner>,
}

/// List team owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_team",
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(TeamsResponse))),
)]
pub async fn get_team_owners(state: AppState, path: CratePath) -> AppResult<Json<TeamsResponse>> {
    let mut conn = state.db_read().await?;
    let krate = path.load_crate(&mut conn).await?;

    let teams = Team::owning(&krate, &mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(Json(TeamsResponse { teams }))
}

/// List user owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_user",
    params(CratePath),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(UsersResponse))),
)]
pub async fn get_user_owners(state: AppState, path: CratePath) -> AppResult<Json<UsersResponse>> {
    let mut conn = state.db_read().await?;

    let krate = path.load_crate(&mut conn).await?;

    let users = User::owning(&krate, &mut conn)
        .await?
        .into_iter()
        .map(Owner::into)
        .collect::<Vec<EncodableOwner>>();

    Ok(Json(UsersResponse { users }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModifyResponse {
    /// A message describing the result of the operation.
    #[schema(example = "user ghost has been invited to be an owner of crate serde")]
    pub msg: String,

    #[schema(example = true)]
    pub ok: bool,
}

/// Add crate owners.
#[utoipa::path(
    put,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    request_body = inline(ChangeOwnersRequest),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(ModifyResponse))),
)]
pub async fn add_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<Json<ModifyResponse>> {
    modify_owners(app, path.name, parts, body, true).await
}

/// Remove crate owners.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    request_body = inline(ChangeOwnersRequest),
    security(
        ("api_token" = []),
        ("cookie" = []),
    ),
    tag = "owners",
    responses((status = 200, description = "Successful Response", body = inline(ModifyResponse))),
)]
pub async fn remove_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<Json<ModifyResponse>> {
    modify_owners(app, path.name, parts, body, false).await
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChangeOwnersRequest {
    /// List of owner login names to add or remove.
    ///
    /// For users, use just the username (e.g., `"octocat"`).
    /// For GitHub teams, use the format `github:org:team` (e.g., `"github:rust-lang:owners"`).
    #[schema(example = json!(["octocat", "github:rust-lang:owners"]))]
    #[serde(alias = "users")]
    owners: Vec<String>,
}

async fn modify_owners(
    app: AppState,
    crate_name: String,
    parts: Parts,
    body: ChangeOwnersRequest,
    add: bool,
) -> AppResult<Json<ModifyResponse>> {
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

    let (msg, emails) = conn
        .transaction(|conn| {
            let app = app.clone();
            async move {
                let krate: Crate = Crate::by_name(&crate_name)
                    .first(conn)
                    .await
                    .optional()?
                    .ok_or_else(|| crate_not_found(&crate_name))?;

                let owners = krate.owners(conn).await?;

                match Rights::get(user, &*app.github, &owners, &app.config.gh_token_encryption).await? {
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
                                    let email = EmailMessage::from_template(
                                        "owner_invite",
                                        context! {
                                            inviter => user.gh_login,
                                            domain => app.emails.domain,
                                            crate_name => krate.name,
                                            token => token.expose_secret()
                                        },
                                    );

                                    match email {
                                        Ok(email_msg) => emails.push((recipient, email_msg)),
                                        Err(error) => warn!("Failed to render owner invite email template: {error}"),
                                    }
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
    for (recipient, email) in emails {
        if let Err(error) = app.emails.send(&recipient, email).await {
            warn!("Failed to send owner invite email to {recipient}: {error}");
        }
    }

    Ok(Json(ModifyResponse { msg, ok: true }))
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
        let encryption = &app.config.gh_token_encryption;
        add_team_owner(&*app.github, conn, req_user, krate, login, encryption).await
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
    let mut users = User::find_all_by_login(conn, login).await?.into_iter();

    let user = users
        .next()
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
    encryption: &GitHubTokenEncryption,
) -> Result<NewOwnerInvite, OwnerAddError> {
    // github:rust-lang:owners
    let mut chunks = login.split(':');

    let team_system = chunks.next().unwrap();
    if team_system != "github" {
        let error = "unknown organization handler, only 'github:org:team' is supported";
        return Err(bad_request(error).into());
    }

    // unwrap is documented above as part of the calling contract
    let org = chunks.next().unwrap();
    let team = chunks.next().ok_or_else(|| {
        let error = "missing github team argument; format is github:org:team";
        bad_request(error)
    })?;

    // Always recreate teams to get the most up-to-date GitHub ID
    let team = create_or_update_github_team(
        gh_client,
        conn,
        &login.to_lowercase(),
        org,
        team,
        req_user,
        encryption,
    )
    .await?;

    // Teams are added as owners immediately, since the above call ensures
    // the user is a team member.
    CrateOwner::builder()
        .crate_id(krate.id)
        .team_id(team.id)
        .created_by(req_user.id)
        .build()
        .insert(conn)
        .await?;

    Ok(NewOwnerInvite::Team(team))
}

/// Tries to create or update a GitHub Team. Assumes `org` and `team` are
/// correctly parsed out of the full `name`. `name` is passed as a
/// convenience to avoid rebuilding it.
pub async fn create_or_update_github_team(
    gh_client: &dyn GitHubClient,
    conn: &mut AsyncPgConnection,
    login: &str,
    org_name: &str,
    team_name: &str,
    req_user: &User,
    encryption: &GitHubTokenEncryption,
) -> AppResult<Team> {
    // GET orgs/:org/teams
    // check that `team` is the `slug` in results, and grab its data

    // "sanitization"
    fn is_allowed_char(c: char) -> bool {
        matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
    }

    if let Some(c) = org_name.chars().find(|c| !is_allowed_char(*c)) {
        return Err(bad_request(format_args!(
            "organization cannot contain special \
                 characters like {c}"
        )));
    }

    let token = encryption
        .decrypt(&req_user.gh_encrypted_token)
        .map_err(|err| {
            custom(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to decrypt GitHub token: {err}"),
            )
        })?;
    let team = gh_client.team_by_name(org_name, team_name, &token).await
        .map_err(|_| {
            bad_request(format_args!(
                "could not find the github team {org_name}/{team_name}. \
                    Make sure that you have the right permissions in GitHub. \
                    See https://doc.rust-lang.org/cargo/reference/publishing.html#github-permissions"
            ))
        })?;

    let org_id = team.organization.id;
    let gh_login = &req_user.gh_login;

    let is_team_member = gh_client
        .team_membership(org_id, team.id, gh_login, &token)
        .await?
        .is_some_and(|m| m.is_active());

    let can_add_team =
        is_team_member || is_gh_org_owner(gh_client, org_id, gh_login, &token).await?;

    if !can_add_team {
        return Err(custom(
            StatusCode::FORBIDDEN,
            "only members of a team or organization owners can add it as an owner",
        ));
    }

    let org = gh_client.org_by_name(org_name, &token).await?;

    NewTeam::builder()
        .login(&login.to_lowercase())
        .org_id(org_id)
        .github_id(team.id)
        .maybe_name(team.name.as_deref())
        .maybe_avatar(org.avatar_url.as_deref())
        .build()
        .create_or_update(conn)
        .await
        .map_err(Into::into)
}

async fn is_gh_org_owner(
    gh_client: &dyn GitHubClient,
    org_id: i32,
    gh_login: &str,
    token: &AccessToken,
) -> Result<bool, GitHubError> {
    let membership = gh_client.org_membership(org_id, gh_login, token).await?;
    Ok(membership.is_some_and(|m| m.is_active_admin()))
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
