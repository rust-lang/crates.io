use crate::app::AppState;
use crate::controllers::helpers::OkResponse;
use crate::email::EmailMessage;
use crate::email::Emails;
use crate::middleware::log_request::RequestLogExt;
use crate::models::{NewEmail, NewOauthGithub, NewUser, OauthGithub};
use crate::schema::{oauth_github, users};
use crate::util::diesel::is_read_only_error;
use crate::util::errors::{AppResult, BoxedAppError, bad_request, not_found, server_error};
use crate::util::no_store;
use crate::util::oauth::ReqwestClient;
use crate::views::EncodableMe;
use axum::Json;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use crates_io_github::{GitHubAuth, GitHubUser};
use crates_io_session::SessionExtension;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use lettre::Address;
use minijinja::context;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

/// Session key containing serialized pending-signup state.
pub const PENDING_SIGNUP_KEY: &str = "pending_signup";
const PENDING_SIGNUP_LIFETIME: chrono::TimeDelta = chrono::TimeDelta::minutes(30);
const PENDING_SIGNUP_ERROR: &str =
    "Your signup session is missing or has expired. Please authenticate with GitHub again.";

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
    responses(
        (status = 200, description = "Successful Response", body = inline(BeginResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn begin_session(app: AppState, session: SessionExtension) -> Json<BeginResponse> {
    session.remove(PENDING_SIGNUP_KEY);

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

/// The result of completing the GitHub OAuth flow.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthorizeResponse {
    /// The GitHub OAuth flow signed in a crates.io user.
    SignedIn(#[schema(inline)] EncodableMe),
    /// The GitHub account needs a crates.io account.
    SignupRequired,
}

/// GitHub account details retained while a user completes signup.
#[derive(Debug, Deserialize, Serialize)]
pub struct PendingSignup {
    /// GitHub-controlled account details captured during OAuth authorization.
    pub github_user: GitHubUser,
    /// Encrypted GitHub access token used when the account is created.
    pub encrypted_token: Vec<u8>,
    /// Creation time used to determine when this pending signup expires.
    pub created_at: DateTime<Utc>,
}

impl PendingSignup {
    /// Creates pending signup state with the current time.
    fn new(github_user: GitHubUser, encrypted_token: Vec<u8>) -> Self {
        Self {
            github_user,
            encrypted_token,
            created_at: Utc::now(),
        }
    }

    /// Returns whether this pending signup has reached its absolute expiry.
    fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now.signed_duration_since(self.created_at) >= PENDING_SIGNUP_LIFETIME
    }
}

/// Public GitHub account details displayed while a user completes signup.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SignupDetails {
    /// The GitHub account login.
    login: String,
    /// The email address suggested by GitHub, if one is available.
    email: Option<String>,
}

/// Response containing details for a pending signup.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SignupResponse {
    signup: SignupDetails,
}

/// Request to complete a pending signup.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CompletePendingSignupRequest {
    /// User-controlled signup details.
    #[schema(inline)]
    signup: CompletePendingSignupData,
}

/// User-controlled details for a pending signup.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CompletePendingSignupData {
    /// The email address to associate with the new account.
    #[schema(value_type = String, format = Email, example = "new-user@example.com")]
    email: Address,
}

/// Loads valid pending-signup state from the signed session cookie.
fn load_pending_signup(session: &SessionExtension) -> AppResult<PendingSignup> {
    let Some(value) = session.get(PENDING_SIGNUP_KEY) else {
        return Err(bad_request(PENDING_SIGNUP_ERROR));
    };

    let pending_signup: PendingSignup = serde_json::from_str(&value).map_err(|_| {
        session.remove(PENDING_SIGNUP_KEY);
        bad_request(PENDING_SIGNUP_ERROR)
    })?;

    if pending_signup.is_expired(Utc::now()) {
        session.remove(PENDING_SIGNUP_KEY);
        return Err(bad_request(PENDING_SIGNUP_ERROR));
    }

    Ok(pending_signup)
}

/// Load the GitHub account details for a pending signup.
///
/// The encrypted GitHub token remains in the signed session cookie and is never returned to the
/// frontend. Missing, malformed, and expired signup state require restarting GitHub
/// authentication.
#[utoipa::path(
    get,
    path = "/api/private/session/signup",
    tag = "session",
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(SignupResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get_pending_signup(
    app: AppState,
    session: SessionExtension,
) -> AppResult<impl IntoResponse> {
    if !app.config.features.explicit_signup_enabled {
        return Err(not_found());
    }
    if session.get("user_id").is_some() {
        return Err(bad_request("You are already signed in."));
    }

    let pending_signup = load_pending_signup(&session)?;

    let json = Json(SignupResponse {
        signup: SignupDetails {
            login: pending_signup.github_user.login,
            email: pending_signup.github_user.email,
        },
    });

    Ok((no_store(), json))
}

/// Complete a pending signup.
#[utoipa::path(
    post,
    path = "/api/private/session/signup",
    request_body = inline(CompletePendingSignupRequest),
    tag = "session",
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(EncodableMe)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn complete_pending_signup(
    app: AppState,
    session: SessionExtension,
    Json(body): Json<CompletePendingSignupRequest>,
) -> AppResult<Json<EncodableMe>> {
    if !app.config.features.explicit_signup_enabled {
        return Err(not_found());
    }
    if session.get("user_id").is_some() {
        return Err(bad_request("You are already signed in."));
    }

    let pending_signup = load_pending_signup(&session)?;
    let mut github_user = pending_signup.github_user;
    github_user.email = Some(body.signup.email.to_string());

    let mut conn = app.db_write().await?;
    let (user_id, user) = conn
        .transaction(async |conn| {
            let user_id = create_user(
                &github_user,
                &pending_signup.encrypted_token,
                &app.emails,
                conn,
            )
            .await?;
            let user = super::user::me::authenticated_user(conn, user_id).await?;
            Ok::<_, BoxedAppError>((user_id, user))
        })
        .await?;

    session.remove(PENDING_SIGNUP_KEY);
    session.insert("user_id".to_string(), user_id.to_string());
    Ok(user)
}

/// Cancel a pending signup.
#[utoipa::path(
    delete,
    path = "/api/private/session/signup",
    tag = "session",
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(OkResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn delete_pending_signup(session: SessionExtension) -> OkResponse {
    session.remove(PENDING_SIGNUP_KEY);
    OkResponse::new()
}

/// Complete authentication flow.
///
/// This route is called from the GitHub API OAuth flow after the user accepted or rejected
/// the data access permissions. It will check the `state` parameter and then call the GitHub API
/// to exchange the temporary `code` for an API token. Existing crates.io users are signed in.
/// New users must complete signup when explicit signup is enabled and are created immediately
/// otherwise.
///
/// see <https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site>
#[utoipa::path(
    post,
    path = "/api/private/session/authorize",
    tag = "session",
    request_body = inline(AuthorizeBody),
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(AuthorizeResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn authorize_session(
    app: AppState,
    session: SessionExtension,
    req: Parts,
    Json(body): Json<AuthorizeBody>,
) -> AppResult<Json<AuthorizeResponse>> {
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

    // Fetch the user info from GitHub using the access token we just got and create a user record
    let auth = GitHubAuth::bearer(token.secret().clone());
    let ghuser = app.github.current_user(&auth).await?;

    let mut conn = app.db_write().await?;
    let user_id = save_user_to_database(
        app.config.features.explicit_signup_enabled,
        &ghuser,
        &encrypted_token,
        &app.emails,
        &mut conn,
    )
    .await?;

    match user_id {
        Some(user_id) => {
            session.remove(PENDING_SIGNUP_KEY);
            session.insert("user_id".to_string(), user_id.to_string());

            let Json(user) = super::user::me::authenticated_user(&mut conn, user_id).await?;
            Ok(Json(AuthorizeResponse::SignedIn(user)))
        }
        None => {
            let pending_signup = PendingSignup::new(ghuser, encrypted_token);
            let pending_signup = serde_json::to_string(&pending_signup)?;
            session.remove("user_id");
            session.insert(PENDING_SIGNUP_KEY.to_string(), pending_signup);
            Ok(Json(AuthorizeResponse::SignupRequired))
        }
    }
}

/// Updates a GitHub-linked user or creates one when explicit signup is disabled.
///
/// Returns `None` when explicit signup is enabled and no existing user was found.
pub async fn save_user_to_database(
    explicit_signup_enabled: bool,
    gh_user: &GitHubUser,
    encrypted_token: &[u8],
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Option<i32>> {
    // There should not be one transaction around both the `create_or_update_user` call and the
    // `find_user_by_gh_id` call. If they're in one transaction and we're in read only mode, the
    // entire transaction will be poisoned and the `find_user_by_gh_id` will fail too, thus
    // negating the purpose of the fallback. There _is_ a transaction around the body of
    // `create_or_update_user`.
    match create_or_update_user(
        explicit_signup_enabled,
        gh_user,
        encrypted_token,
        emails,
        conn,
    )
    .await
    {
        Ok(user_id) => Ok(user_id),
        Err(error) if is_read_only_error(&error) => {
            // In read-only mode, we can't update users or create new ones.
            // Look up the GitHub account to distinguish an existing user from a new signup.
            match find_user_by_gh_id(conn, gh_user.id).await? {
                Some(user_id) => Ok(Some(user_id)),
                None if explicit_signup_enabled => Ok(None),
                None => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

/// Updates an existing user or optionally inserts a new user within a transaction.
///
/// Returns `None` when explicit signup is enabled and no existing user was found.
async fn create_or_update_user(
    explicit_signup_enabled: bool,
    gh_user: &GitHubUser,
    encrypted_token: &[u8],
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Option<i32>> {
    conn.transaction(async |conn| {
        let update_result = update_user(gh_user, encrypted_token, conn).await;

        match update_result {
            Ok(user_id) => Ok(Some(user_id)),
            Err(diesel::result::Error::NotFound) if explicit_signup_enabled => Ok(None),
            Err(diesel::result::Error::NotFound) => {
                // If the update fails because the `oauth_github` record doesn't exist, this
                // currently means the `user` record doesn't exist either and we need to create
                // both. This assumption holds because crates.io and GitHub accounts currently have
                // a one-to-one relationship; this will need to be changed if/when we allow
                // crates.io users to link more than one GitHub account to their crates.io account.
                create_user(gh_user, encrypted_token, emails, conn)
                    .await
                    .map(Some)
            }
            Err(error) => Err(error),
        }
    })
    .await
}

/// Updates an existing user. Should be called in a transaction, as `create_or_update_user` does,
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
        ))
        .execute(conn)
        .await?;

    Ok(oauth_github.user_id)
}

/// Inserts a new user into the database.
///
/// This method also inserts the email address into the `emails` table
/// and sends a confirmation email to the user.
///
/// Should be called in a transaction so the `users`, `oauth_github`, and `emails` records are
/// inserted together.
async fn create_user(
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
    responses(
        (status = 200, description = "Successful Response", body = inline(OkResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn end_session(session: SessionExtension) -> OkResponse {
    session.remove("user_id");
    session.remove(PENDING_SIGNUP_KEY);
    OkResponse::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{EncodablePrivateUser, OwnedCrate};
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use crates_io_test_db::TestDatabase;
    use diesel_async::RunQueryDsl;
    use insta::assert_json_snapshot;

    fn github_user() -> GitHubUser {
        GitHubUser {
            avatar_url: Some("https://avatars.example.com/42".into()),
            email: Some("ghost@example.com".into()),
            id: 42,
            login: "ghost".into(),
            name: Some("Ghost".into()),
        }
    }

    #[test]
    fn pending_signup_expires_after_thirty_minutes() {
        let now = Utc::now();
        let pending_signup = PendingSignup {
            github_user: github_user(),
            encrypted_token: vec![1, 2, 3],
            created_at: now - chrono::TimeDelta::minutes(30),
        };

        assert!(pending_signup.is_expired(now));
    }

    #[test]
    fn pending_signup_is_valid_before_thirty_minutes() {
        let now = Utc::now();
        let pending_signup = PendingSignup {
            github_user: github_user(),
            encrypted_token: vec![1, 2, 3],
            created_at: now - chrono::TimeDelta::minutes(29),
        };

        assert!(!pending_signup.is_expired(now));
    }

    async fn count_users(conn: &mut AsyncPgConnection) -> i64 {
        users::table.count().get_result(conn).await.unwrap()
    }

    async fn count_oauth_github(conn: &mut AsyncPgConnection) -> i64 {
        oauth_github::table.count().get_result(conn).await.unwrap()
    }

    #[tokio::test]
    async fn explicit_signup_does_not_create_new_user() {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user_id = assert_ok!(
            save_user_to_database(true, &github_user(), &[1, 2, 3], &emails, &mut conn).await
        );

        assert_none!(user_id);
        assert_eq!(count_users(&mut conn).await, 0);
        assert_eq!(count_oauth_github(&mut conn).await, 0);
    }

    #[tokio::test]
    async fn explicit_signup_signs_in_existing_user() {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user = github_user();
        let user_id = assert_some!(assert_ok!(
            save_user_to_database(false, &user, &[1], &emails, &mut conn).await
        ));

        let actual_user_id =
            assert_ok!(save_user_to_database(true, &user, &[2], &emails, &mut conn).await);

        assert_some_eq!(actual_user_id, user_id);
        assert_eq!(count_users(&mut conn).await, 1);
    }

    #[tokio::test]
    async fn legacy_signup_creates_and_signs_in_new_user() {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user_id = assert_ok!(
            save_user_to_database(false, &github_user(), &[1, 2, 3], &emails, &mut conn).await
        );

        assert_some!(user_id);
        assert_eq!(count_users(&mut conn).await, 1);
        assert_eq!(count_oauth_github(&mut conn).await, 1);
    }

    #[tokio::test]
    async fn explicit_signup_detects_new_user_during_read_only_mode() {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        assert_ok!(
            diesel::sql_query("SET default_transaction_read_only = 't'")
                .execute(&mut conn)
                .await
        );

        let user_id = assert_ok!(
            save_user_to_database(true, &github_user(), &[1, 2, 3], &emails, &mut conn).await
        );

        assert_none!(user_id);
    }

    #[tokio::test]
    async fn explicit_signup_signs_in_existing_user_during_read_only_mode() {
        let emails = Emails::new_in_memory();
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user = github_user();
        let user_id = assert_some!(assert_ok!(
            save_user_to_database(false, &user, &[1], &emails, &mut conn).await
        ));
        assert_ok!(
            diesel::sql_query("SET default_transaction_read_only = 't'")
                .execute(&mut conn)
                .await
        );

        let actual_user_id =
            assert_ok!(save_user_to_database(true, &user, &[2], &emails, &mut conn).await);

        assert_some_eq!(actual_user_id, user_id);
    }

    #[test]
    fn authorize_response_serializes_signed_in() {
        let response = AuthorizeResponse::SignedIn(EncodableMe {
            user: EncodablePrivateUser {
                id: 42,
                login: "ghost".into(),
                email_verified: true,
                email_verification_sent: true,
                name: Some("Kate Morgan".into()),
                email: Some("kate@morgan.dev".into()),
                avatar: Some("https://avatars2.githubusercontent.com/u/1234567?v=4".into()),
                url: "https://github.com/ghost".into(),
                is_admin: false,
                publish_notifications: true,
                created_at: None,
            },
            owned_crates: vec![OwnedCrate {
                id: 123,
                name: "serde".into(),
                email_notifications: true,
            }],
        });

        assert_json_snapshot!(response, @r#"
        {
          "status": "signed_in",
          "user": {
            "id": 42,
            "login": "ghost",
            "email_verified": true,
            "email_verification_sent": true,
            "name": "Kate Morgan",
            "email": "kate@morgan.dev",
            "avatar": "https://avatars2.githubusercontent.com/u/1234567?v=4",
            "url": "https://github.com/ghost",
            "is_admin": false,
            "publish_notifications": true,
            "created_at": null
          },
          "owned_crates": [
            {
              "id": 123,
              "name": "serde",
              "email_notifications": true
            }
          ]
        }
        "#);
    }

    #[test]
    fn authorize_response_serializes_signup_required() {
        assert_json_snapshot!(AuthorizeResponse::SignupRequired, @r#"
        {
          "status": "signup_required"
        }
        "#);
    }

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

        let result = save_user_to_database(false, &gh_user, &[], &emails, &mut conn).await;

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {result:?}"
        );
    }
}
