use crate::app::AppState;
use crate::email::EmailMessage;
use crate::email::Emails;
use crate::middleware::log_request::RequestLogExt;
use crate::models::{NewEmail, NewOauthGithub, NewUser, OauthGithub};
use crate::schema::{oauth_github, users};
use crate::util::diesel::is_read_only_error;
use crate::util::errors::{AppResult, bad_request, server_error};
use crate::util::oauth::ReqwestClient;
use crate::views::{EncodableMe, EncodablePrivateUser};
use axum::Json;
use chrono::Utc;
use crates_io_github::{GitHubAuth, GitHubUser};
use crates_io_session::SessionExtension;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use minijinja::context;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BeginResponse {
    #[schema(
        example = "https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg"
    )]
    pub url: String,

    #[schema(example = "b84a63c4ea3fcb4ac84")]
    pub state: String,
}

/// Begin authentication flow.
///
/// This route will return an authorization URL for the GitHub OAuth flow including the crates.io
/// `client_id` and a randomly generated `state` secret.
///
/// see <https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access>
#[utoipa::path(
    post,
    path = "/api/private/session/begin",
    tag = "session",
    extensions(("x-internal" = json!(true))),
    responses((status = 200, description = "Successful Response", body = inline(BeginResponse))),
)]
pub async fn begin_session(app: AppState, session: SessionExtension) -> Json<BeginResponse> {
    let (url, state) = app
        .github_oauth
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(Scope::new("read:org".to_string()))
        .url();

    let state = state.secret().to_string();
    session.insert("github_oauth_state".to_string(), state.clone());

    let url = url.to_string();
    Json(BeginResponse { url, state })
}

#[derive(Clone, Debug, Deserialize, utoipa::ToSchema)]
pub struct AuthorizeBody {
    /// Temporary code received from the GitHub API.
    #[schema(value_type = String, example = "901dd10e07c7e9fa1cd5")]
    code: AuthorizationCode,
    /// State parameter received from the GitHub API.
    #[schema(value_type = String, example = "fYcUY3FMdUUz00FC7vLT7A")]
    state: CsrfToken,
}

/// Complete authentication flow.
///
/// This route is called from the GitHub API OAuth flow after the user accepted or rejected
/// the data access permissions. It will check the `state` parameter and then call the GitHub API
/// to exchange the temporary `code` for an API token. The API token is returned together with
/// the corresponding user information.
///
/// see <https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site>
#[utoipa::path(
    post,
    path = "/api/private/session/authorize",
    tag = "session",
    request_body = inline(AuthorizeBody),
    extensions(("x-internal" = json!(true))),
    responses((status = 200, description = "Successful Response", body = inline(EncodableMe))),
)]
pub async fn authorize_session(
    app: AppState,
    session: SessionExtension,
    req: Parts,
    Json(body): Json<AuthorizeBody>,
) -> AppResult<Json<EncodableMe>> {
    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    let session_state = session.remove("github_oauth_state").map(CsrfToken::new);
    if session_state.is_none_or(|session_state| body.state.secret() != session_state.secret()) {
        return Err(bad_request("invalid state parameter"));
    }

    // Fetch the access token from GitHub using the code we just got
    let client = ReqwestClient(
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
    );

    let token = app
        .github_oauth
        .exchange_code(body.code)
        .request_async(&client)
        .await
        .map_err(|err| {
            req.request_log().add("cause", err);
            server_error("Error obtaining token")
        })?;

    let token = token.access_token();

    // Encrypt the GitHub access token
    let encryption = &app.config.token_encryption;
    let encrypted_token = encryption.encrypt(token.secret()).map_err(|error| {
        error!("Failed to encrypt GitHub token: {error}");
        server_error("Internal server error")
    })?;

    // Fetch the user info from GitHub using the access token we just got
    let auth = GitHubAuth::bearer(token.secret().clone());
    let gh_user = app.github.current_user(&auth).await?;

    let mut conn = app.db_write().await?;
    // Try to log in an existing user
    match sign_in_existing_user(&gh_user, &encrypted_token, &mut conn).await? {
        Some(user_id) => {
            // Log in by setting a cookie and the middleware authentication
            session.insert("user_id".to_string(), user_id.to_string());
            super::user::me::authenticated_user(&mut conn, user_id).await
        }
        None => {
            // If the user doesn't exist in the database yet, confirm their information before
            // creating their account. Carry the GitHub user info and the encrypted GitHub token
            // along in the session cookie.
            session.insert("github_user".to_string(), serde_json::to_string(&gh_user)?);
            session.insert("encrypted_token".to_string(), hex::encode(&encrypted_token));

            Ok(Json(EncodableMe {
                user: EncodablePrivateUser {
                    // Send an invalid crates.io ID because we haven't saved this user in the
                    // database yet. The frontend will show the "complete signup" form.
                    id: -1,
                    login: gh_user.login.clone(),
                    email_verified: false,
                    email_verification_sent: false,
                    name: gh_user.name,
                    email: gh_user.email,
                    avatar: gh_user.avatar_url,
                    url: Some(format!("https://github.com/{}", gh_user.login)),
                    is_admin: false,
                    publish_notifications: false,
                    created_at: None,
                },
                owned_crates: Default::default(),
            }))
        }
    }
}

pub async fn sign_in_existing_user(
    gh_user: &GitHubUser,
    encrypted_token: &[u8],
    conn: &mut AsyncPgConnection,
) -> QueryResult<Option<i32>> {
    // There should not be one transaction around both the `update_user` call and the
    // `find_user_by_gh_id` call. If they're in one transaction and we're in read only mode, the
    // entire transaction will be poisoned and the `find_user_by_gh_id` will fail too, thus
    // negating the purpose of the fallback.
    match conn
        .transaction(async |conn| update_user(gh_user, encrypted_token, conn).await)
        .await
    {
        Ok(id) => Ok(Some(id)),
        Err(error) if is_read_only_error(&error) => {
            // If we're in read only mode, we can't update their details.
            // just look for an existing user
            find_user_by_gh_id(conn, gh_user.id).await
        }
        Err(diesel::result::Error::NotFound) => {
            // If the update fails because the `oauth_github` record doesn't exist, this
            // currently means the `user` record doesn't exist either and we need to create
            // both. This assumption holds because crates.io and GitHub accounts currently have
            // a one-to-one relationship; this will need to be changed if/when we allow
            // crates.io users to link more than one GitHub account to their crates.io account.
            // Return `None` to signify that this user needs to sign up.
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// Updates an existing user. Should be called in a transaction, as `sign_in_existing_user` does,
/// so both the `users` and `oauth_github` records are updated or neither are.
///
/// Returns an error if the `oauth_github` or `users` records don't exist.
async fn update_user(
    gh_user: &GitHubUser,
    encrypted_token: &[u8],
    conn: &mut AsyncPgConnection,
) -> QueryResult<i32> {
    // First, try to update an existing `oauth_github` record with the specified GitHub ID
    // and the associated `users` record.
    //
    // For now, update user display name, gh_login, and username. Eventually, we will
    // get rid of `gh_login` and stop syncing `name` and `username` with GitHub.
    let oauth_github = diesel::update(oauth_github::table)
        .filter(oauth_github::account_id.eq(gh_user.id as i64))
        .set((
            oauth_github::encrypted_token.eq(encrypted_token),
            oauth_github::login.eq(&gh_user.login),
            oauth_github::avatar.eq(gh_user.avatar_url.as_deref()),
            oauth_github::last_sync.eq(Utc::now()),
        ))
        .get_result::<OauthGithub>(conn)
        .await?;
    diesel::update(users::table)
        .filter(users::id.eq(oauth_github.user_id))
        .set((
            users::name.eq(gh_user.name.as_ref()),
            users::username.eq(&gh_user.login),
            // These fields are soon to be deprecated.
            users::gh_login.eq(&gh_user.login),
            users::gh_encrypted_token.eq(encrypted_token),
        ))
        .execute(conn)
        .await?;

    Ok(oauth_github.user_id)
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConfirmedUserInfo {
    email: Option<String>,
}

/// Complete account creation.
///
/// This route is called from the frontend after the user confirmed or modified the information
/// from GitHub. The unmodified GitHub information should be in the session cookie; if it isn't,
/// the authentication flow must be restarted.
///
/// If account creation is successful, the user is logged in and their information is returned.
#[utoipa::path(
    post,
    path = "/api/private/session/confirm",
    security(("cookie" = [])),
    tag = "session",
    request_body = inline(ConfirmedUserInfo),
    extensions(("x-internal" = json!(true))),
    responses((status = 200, description = "Successful Response", body = inline(EncodableMe))),
)]
pub async fn complete_signup(
    app: AppState,
    session: SessionExtension,
    Json(confirmed_user_info): Json<ConfirmedUserInfo>,
) -> AppResult<Json<EncodableMe>> {
    let gh_user_json = session
        .remove("github_user")
        .ok_or_else(|| bad_request("missing github_user session value"))?;
    let mut gh_user: GitHubUser = serde_json::from_str(&gh_user_json)
        .map_err(|_| bad_request("invalid github_user session value"))?;

    let encrypted_token_hex = session
        .remove("encrypted_token")
        .ok_or_else(|| bad_request("missing encrypted_token session value"))?;
    let encrypted_token: Vec<u8> = hex::decode(&encrypted_token_hex)
        .map_err(|_| bad_request("invalid encrypted_token session value"))?;

    let ConfirmedUserInfo { email } = confirmed_user_info;

    gh_user.email = email;

    let mut conn = app.db_write().await?;
    let user_id = sign_up_new_user(&gh_user, &encrypted_token, &app.emails, &mut conn).await?;

    // Log in by setting a cookie and the middleware authentication
    session.insert("user_id".to_string(), user_id.to_string());

    super::user::me::authenticated_user(&mut conn, user_id).await
}

/// Inserts a new user into the database.
///
/// This method also inserts the email address into the `emails` table
/// and sends a confirmation email to the user.
///
/// Should be called in a transaction, as `create_or_update_user` does, so both the `users` and
/// `emails` records are inserted or neither are.
pub async fn sign_up_new_user(
    gh_user: &GitHubUser,
    encrypted_token: &[u8],
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<i32> {
    let new_user = NewUser::builder()
        .gh_id(gh_user.id)
        .gh_login(&gh_user.login)
        .username(&gh_user.login)
        .maybe_name(gh_user.name.as_deref())
        .gh_encrypted_token(encrypted_token)
        .build();

    let user_id = new_user.insert(conn).await?;

    let new_oauth_github = NewOauthGithub::builder()
        .user_id(user_id)
        .account_id(gh_user.id as i64)
        .encrypted_token(encrypted_token)
        .login(&gh_user.login)
        .maybe_avatar(gh_user.avatar_url.as_deref())
        .build();

    new_oauth_github.insert(conn).await?;

    // Since this is a new user, send an account verification email
    if let Some(user_email) = gh_user.email.as_deref() {
        let new_email = NewEmail::builder()
            .user_id(user_id)
            .email(user_email)
            .build();

        if let Some(token) = new_email.insert_if_missing(conn).await? {
            let email = EmailMessage::from_template(
                "user_confirm",
                context! {
                    user_name => new_user.gh_login,
                    domain => emails.domain,
                    token => token.expose_secret()
                },
            );

            match email {
                Ok(email) => {
                    // Swallows any error. Users might insert an invalid email address, but
                    // they should still be allowed to create an account; they will need to
                    // fix their email address later.
                    let _ = emails.send(user_email, email).await;
                }
                Err(error) => {
                    warn!("Failed to render user confirmation email template: {error}");
                }
            };
        }
    }

    Ok(user_id)
}

async fn find_user_by_gh_id(mut conn: &AsyncPgConnection, gh_id: i32) -> QueryResult<Option<i32>> {
    users::table
        .inner_join(oauth_github::table)
        .filter(oauth_github::account_id.eq(gh_id as i64))
        .select(users::id)
        .first(&mut conn)
        .await
        .optional()
}

/// End the current session.
#[utoipa::path(
    delete,
    path = "/api/private/session",
    security(("cookie" = [])),
    tag = "session",
    extensions(("x-internal" = json!(true))),
    responses((status = 200, description = "Successful Response")),
)]
pub async fn end_session(session: SessionExtension) -> Json<bool> {
    session.remove("user_id");
    Json(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;

    #[tokio::test]
    async fn gh_user_with_invalid_email_doesnt_fail() {
        let emails = Emails::new_in_memory();

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let gh_user = GitHubUser {
            email: Some("String.Format(\"{0}.{1}@live.com\", FirstName, LastName)".into()),
            name: Some("My Name".into()),
            login: "github_user".into(),
            id: -1,
            avatar_url: None,
        };

        let result = sign_up_new_user(&gh_user, &[], &emails, &mut conn).await;

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {result:?}"
        );
    }
}
