//! All routes related to managing owners of a crate

use crate::controllers::helpers::authorization::Rights;
use crate::controllers::krate::CratePath;
use crate::models::krate::OwnerRemoveError;
use crate::models::{Crate, Owner, PublicUser, Team, User};
use crate::models::{
    CrateOwner, NewCrateOwnerInvitation, NewCrateOwnerInvitationOutcome, NewTeam,
    krate::NewOwnerInvite, token::EndpointScope,
};
use crate::util::errors::{AppResult, BoxedAppError, bad_request, custom, forbidden};
use crate::views::EncodableOwner;
use crate::worker::jobs::SendEmail;
use crate::{App, app::AppState};
use crate::{auth::AuthCheck, email::EmailMessage};
use axum::Json;
use chrono::Utc;
use crates_io_encryption::TokenEncryption;
use crates_io_github::{GitHubAuth, GitHubClient, GitHubError};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use http::StatusCode;
use http::request::Parts;
use minijinja::context;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UsersResponse {
    pub users: Vec<EncodableOwner>,
}

/// Lists crate owners.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owners",
    params(CratePath),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(UsersResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn list_owners(state: AppState, path: CratePath) -> AppResult<Json<UsersResponse>> {
    let conn = state.db_read().await?;

    let krate = path.load_crate(&conn).await?;

    let (mut users, mut teams) = tokio::try_join!(
        PublicUser::owning(&krate, &conn),
        Team::owning(&krate, &conn),
    )?;
    users.sort_by_key(|user| user.id);
    teams.sort_by_key(|team| team.id);

    let users = users
        .into_iter()
        .map(EncodableOwner::from_user)
        .chain(teams.into_iter().map(EncodableOwner::from_team))
        .collect::<Vec<_>>();

    Ok(Json(UsersResponse { users }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TeamsResponse {
    pub teams: Vec<EncodableOwner>,
}

/// Lists team owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_team",
    params(CratePath),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(TeamsResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_team_owners(state: AppState, path: CratePath) -> AppResult<Json<TeamsResponse>> {
    let conn = state.db_read().await?;
    let krate = path.load_crate(&conn).await?;

    let mut teams = Team::owning(&krate, &conn).await?;
    teams.sort_by_key(|team| team.id);

    let teams = teams
        .into_iter()
        .map(EncodableOwner::from_team)
        .collect::<Vec<_>>();

    Ok(Json(TeamsResponse { teams }))
}

/// Lists user owners of a crate.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/owner_user",
    params(CratePath),
    tag = "owners",
    responses(
        (status = 200, description = "Successful Response", body = inline(UsersResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_user_owners(state: AppState, path: CratePath) -> AppResult<Json<UsersResponse>> {
    let conn = state.db_read().await?;

    let krate = path.load_crate(&conn).await?;

    let mut users = PublicUser::owning(&krate, &conn).await?;
    users.sort_by_key(|user| user.id);

    let users = users
        .into_iter()
        .map(EncodableOwner::from_user)
        .collect::<Vec<_>>();

    Ok(Json(UsersResponse { users }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModifyResponse {
    /// A message describing the result of the operation.
    #[schema(example = "Crates.io user ghost has been invited to be an owner of crate serde")]
    pub msg: String,

    #[schema(example = true)]
    pub ok: bool,
}

/// Adds crate owners.
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
    responses(
        (status = 200, description = "Successful Response", body = inline(ModifyResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn add_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
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
        .for_crate(&path.name)
        .check(&parts, &mut conn)
        .await?;

    let user = auth.user();

    let krate = path.load_crate(&conn).await?;

    let owners = krate.owners(&conn).await?;

    check_owner_permissions(&app, user, &owners).await?;

    let msg = conn
        .transaction(async |conn| {
            let mut msgs = Vec::with_capacity(logins.len());
            for login in &logins {
                match add_owner(&app, conn, user, &krate, &owners, login).await {
                    // A user must accept the invitation through their account
                    // or with the emailed token.
                    Ok(NewOwnerInvite::User(invitee, token)) => {
                        msgs.push(format!(
                            "Crates.io user {} has been invited to be an owner of crate {}",
                            invitee.username, krate.name,
                        ));

                        if let Some(recipient) = invitee.verified_email(conn).await.ok().flatten() {
                            let recipient = recipient.parse().map_err(|_| {
                                bad_request(format_args!(
                                    "user `{}` has an invalid email address",
                                    invitee.username
                                ))
                            })?;
                            let email = render_owner_invite_email(&app, user, &krate, &token);

                            match email {
                                Ok(email) => {
                                    SendEmail::new(recipient, email).enqueue(conn).await?;
                                }
                                Err(error) => {
                                    warn!("Failed to render owner invite email template: {error}")
                                }
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
                        "Crates.io user {} already has a pending invitation \
                        to be an owner of crate {}",
                        user.username, krate.name
                    )),

                    // An opaque error occurred.
                    Err(OwnerAddError::Diesel(e)) => return Err(e.into()),
                    Err(OwnerAddError::AppError(e)) => return Err(e),
                }
            }

            Ok(msgs.join(","))
        })
        .await?;

    Ok(Json(ModifyResponse { msg, ok: true }))
}

/// Removes crate owners.
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
    responses(
        (status = 200, description = "Successful Response", body = inline(ModifyResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn remove_owners(
    app: AppState,
    path: CratePath,
    parts: Parts,
    Json(body): Json<ChangeOwnersRequest>,
) -> AppResult<Json<ModifyResponse>> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::ChangeOwners)
        .for_crate(&path.name)
        .check(&parts, &mut conn)
        .await?;

    let user = auth.user();

    let krate = path.load_crate(&conn).await?;

    let owners = krate.owners(&conn).await?;

    check_owner_permissions(&app, user, &owners).await?;

    conn.transaction(async |conn| {
        for login in &body.owners {
            krate.owner_remove(conn, login).await?;
        }
        if User::owning(&krate, conn).await?.is_empty() {
            return Err(bad_request(
                "cannot remove all individual owners of a crate. \
                 Team member don't have permission to modify owners, so \
                 at least one individual owner is required.",
            ));
        }

        Ok(())
    })
    .await?;

    Ok(Json(ModifyResponse {
        msg: "owners successfully removed".to_owned(),
        ok: true,
    }))
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

/// Checks whether the user has permission to modify the crate's owners.
async fn check_owner_permissions(app: &App, user: &User, owners: &[Owner]) -> AppResult<()> {
    const TEAM_MEMBER_ERROR: &str = "team members don't have permission to modify owners";
    const NOT_OWNER_ERROR: &str = "only owners have permission to modify owners";

    match Rights::get(user, &*app.github, owners, &app.config.token_encryption).await? {
        Rights::Full => Ok(()),
        Rights::Publish => Err(forbidden(TEAM_MEMBER_ERROR)),
        Rights::None => Err(forbidden(NOT_OWNER_ERROR)),
    }
}

/// Renders an owner invitation email with a token for accepting the invitation.
fn render_owner_invite_email(
    app: &App,
    inviter: &User,
    krate: &Crate,
    token: &SecretString,
) -> Result<EmailMessage, minijinja::Error> {
    EmailMessage::from_template(
        "owner_invite",
        context! {
            inviter => inviter.gh_login,
            domain => app.emails.domain,
            crate_name => krate.name,
            token => token.expose_secret()
        },
    )
}

/// Invites `login` as an owner of this crate, returning the created
/// [`NewOwnerInvite`].
async fn add_owner(
    app: &App,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    owners: &[Owner],
    login: &str,
) -> Result<NewOwnerInvite, OwnerAddError> {
    match Login::parse(login)? {
        Login::GitHubTeam(team) => {
            let login_test = |owner: &Owner| owner.login().to_lowercase() == *login.to_lowercase();
            if owners.iter().any(login_test) {
                return Err(bad_request(format_args!("`{login}` is already an owner")).into());
            }

            let github = &*app.github;
            let encryption = &app.config.token_encryption;
            add_github_team_owner(github, conn, req_user, krate, team, encryption).await
        }
        Login::Unprefixed(login) => {
            let user = User::find_by_login(conn, login).await.optional()?;
            let user = user.ok_or_else(|| {
                bad_request(format_args!("could not find user with login `{login}`"))
            })?;

            let is_owner =
                |owner: &Owner| matches!(owner, Owner::User(owner) if owner.id == user.id);
            if owners.iter().any(is_owner) {
                return Err(bad_request(format_args!("`{login}` is already an owner")).into());
            }

            invite_user_owner(app, conn, req_user, krate, user).await
        }
    }
}

/// Parsed owner login used by the owner endpoints.
enum Login<'a> {
    /// GitHub organization team, such as `github:rust-lang:owners`.
    GitHubTeam(GitHubTeamLogin<'a>),
    /// User login without a service prefix.
    Unprefixed(&'a str),
}

impl<'a> Login<'a> {
    /// Parses an owner login.
    fn parse(login: &'a str) -> Result<Self, BoxedAppError> {
        if !login.contains(':') {
            return Ok(Self::Unprefixed(login));
        }

        GitHubTeamLogin::parse(login).map(Self::GitHubTeam)
    }
}

/// Parsed GitHub organization team login, such as `github:rust-lang:owners`.
struct GitHubTeamLogin<'a> {
    login: &'a str,
    org: &'a str,
    team: &'a str,
}

impl<'a> GitHubTeamLogin<'a> {
    /// Parses a GitHub organization team login.
    fn parse(login: &'a str) -> Result<Self, BoxedAppError> {
        let mut chunks = login.split(':');
        let team_system = chunks.next().unwrap_or_default();
        if team_system != "github" {
            let error = "unknown organization handler, only 'github:org:team' is supported";
            return Err(bad_request(error));
        }

        let org = chunks.next().unwrap_or_default();
        let team = chunks.next().ok_or_else(|| {
            let error = "missing github team argument; format is github:org:team";
            bad_request(error)
        })?;

        Ok(Self { login, org, team })
    }
}

/// Creates an owner invitation for a resolved user.
async fn invite_user_owner(
    app: &App,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    user: User,
) -> Result<NewOwnerInvite, OwnerAddError> {
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

async fn add_github_team_owner(
    gh_client: &dyn GitHubClient,
    conn: &mut AsyncPgConnection,
    req_user: &User,
    krate: &Crate,
    login: GitHubTeamLogin<'_>,
    encryption: &TokenEncryption,
) -> Result<NewOwnerInvite, OwnerAddError> {
    // Always recreate teams to get the most up-to-date GitHub ID
    let team = create_or_update_github_team(
        gh_client,
        conn,
        &login.login.to_lowercase(),
        login.org,
        login.team,
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
    encryption: &TokenEncryption,
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

    let Some(token) = req_user.gh_encrypted_token.as_ref() else {
        return Err(bad_request(
            "Cannot add a GitHub team as an owner without a connected GitHub account",
        ));
    };

    let token = encryption.decrypt(token).map_err(|err| {
        custom(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to decrypt GitHub token: {err}"),
        )
    })?;

    let auth = GitHubAuth::bearer(token);
    let team = gh_client.team_by_name(org_name, team_name, &auth).await
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
        .team_membership(org_id, team.id, gh_login, &auth)
        .await?
        .is_some_and(|m| m.is_active());

    let can_add_team =
        is_team_member || is_gh_org_owner(gh_client, org_id, gh_login, &auth).await?;

    if !can_add_team {
        return Err(custom(
            StatusCode::FORBIDDEN,
            "only members of a team or organization owners can add it as an owner",
        ));
    }

    let org = gh_client.org_by_name(org_name, &auth).await?;

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
    auth: &GitHubAuth,
) -> Result<bool, GitHubError> {
    let membership = gh_client.org_membership(org_id, gh_login, auth).await?;
    Ok(membership.is_some_and(|m| m.is_active_admin()))
}

/// Error results from an [`add_owner()`] call.
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

#[cfg(test)]
mod tests {
    use super::{GitHubTeamLogin, Login};

    #[test]
    fn parses_github_team_login() {
        let login = GitHubTeamLogin::parse("github:rust-lang:owners").unwrap();

        assert_eq!(login.login, "github:rust-lang:owners");
        assert_eq!(login.org, "rust-lang");
        assert_eq!(login.team, "owners");
    }

    #[test]
    fn parses_unprefixed_login() {
        let login = Login::parse("octocat").unwrap();

        let Login::Unprefixed(username) = login else {
            panic!("expected unprefixed login");
        };
        assert_eq!(username, "octocat");
    }

    #[test]
    fn parses_github_team_as_login() {
        let login = Login::parse("github:rust-lang:owners").unwrap();

        let Login::GitHubTeam(team) = login else {
            panic!("expected GitHub team login");
        };
        assert_eq!(team.org, "rust-lang");
        assert_eq!(team.team, "owners");
    }
}
