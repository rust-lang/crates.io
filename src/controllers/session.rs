use crate::app::AppState;
use crate::email::EmailMessage;
use crate::email::Emails;
use crate::middleware::log_request::RequestLogExt;
use crate::models::{NewEmail, NewOauthGithub, NewUser, User};
use crate::oauth::provider::{ProviderError, UserInfo};
use crate::schema::users;
use crate::util::diesel::is_read_only_error;
use crate::util::errors::{AppResult, BoxedAppError, bad_request, not_found, server_error};
use crate::views::EncodableMe;
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use crates_io_github::GitHubUser;
use crates_io_session::SessionExtension;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use minijinja::context;
use oauth2::{AuthorizationCode, CsrfToken};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

/// Session key for the OAuth state payload (used during the OAuth dance).
pub const SESSION_KEY_OAUTH_STATE: &str = "oauth_state";

/// Session key for the logged-in user id.
pub const SESSION_KEY_USER_ID: &str = "user_id";

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BeginResponse {
    #[schema(
        example = "https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg"
    )]
    pub url: String,

    #[schema(example = "b84a63c4ea3fcb4ac84")]
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct BeginQuery {
    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String {
    crate::oauth::github_provider::PROVIDER_NAME.to_string()
}

/// The JSON payload stored in the session under `"oauth_state"`.
#[derive(Debug, Serialize, Deserialize)]
struct OAuthStatePayload {
    state: String,
    provider: String,
}

/// Begin authentication flow.
///
/// This route will return an authorization URL for the OAuth flow including the crates.io
/// `client_id` and a randomly generated `state` secret.
///
/// An optional `?provider=<name>` query param selects the OAuth provider (default: `"github"`).
///
/// see <https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access>
#[utoipa::path(
    get,
    path = "/api/private/session/begin",
    tag = "session",
    responses((status = 200, description = "Successful Response", body = inline(BeginResponse))),
)]
pub async fn begin_session(
    app: AppState,
    Query(query): Query<BeginQuery>,
    session: SessionExtension,
) -> AppResult<Json<BeginResponse>> {
    let provider = app
        .oauth_providers
        .get(&query.provider)
        .ok_or_else(not_found)?;

    let (url, csrf) = provider.authorize_url();

    let payload = OAuthStatePayload {
        state: csrf.secret().to_string(),
        provider: query.provider,
    };
    session.insert(
        SESSION_KEY_OAUTH_STATE.to_string(),
        serde_json::to_string(&payload).map_err(|e| {
            error!("Failed to serialize OAuth state payload: {e}");
            server_error("Internal server error")
        })?,
    );

    let url = url.to_string();
    Ok(Json(BeginResponse {
        url,
        state: payload.state,
    }))
}

#[derive(Clone, Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct AuthorizeQuery {
    /// Temporary code received from the OAuth provider.
    #[param(value_type = String, example = "901dd10e07c7e9fa1cd5")]
    code: AuthorizationCode,
    /// State parameter received from the OAuth provider (CSRF token).
    #[param(value_type = String, example = "fYcUY3FMdUUz00FC7vLT7A")]
    state: CsrfToken,
}

/// Complete authentication flow.
///
/// This route is called from the OAuth provider after the user accepted or rejected
/// the data access permissions. It will check the `state` parameter and then call the provider
/// API to exchange the temporary `code` for an API token. The API token is returned together with
/// the corresponding user information.
///
/// see <https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site>
///
/// ## Query Parameters
///
/// - `code` – temporary code received from the OAuth provider  **(Required)**
/// - `state` – state parameter received from the OAuth provider  **(Required)**
#[utoipa::path(
    get,
    path = "/api/private/session/authorize",
    tag = "session",
    params(AuthorizeQuery),
    responses((status = 200, description = "Successful Response", body = inline(EncodableMe))),
)]
pub async fn authorize_session(
    query: AuthorizeQuery,
    app: AppState,
    session: SessionExtension,
    req: Parts,
) -> AppResult<Json<EncodableMe>> {
    // Read and parse the session state payload set during `begin_session`.
    let raw_payload = session
        .remove(SESSION_KEY_OAUTH_STATE)
        .ok_or_else(|| bad_request("invalid state parameter"))?;

    let payload: OAuthStatePayload =
        serde_json::from_str(&raw_payload).map_err(|_| bad_request("invalid state parameter"))?;

    // Validate CSRF: the `state` query param must match the stored CSRF token.
    if query.state.secret() != &payload.state {
        return Err(bad_request("invalid state parameter"));
    }

    let provider = app
        .oauth_providers
        .get(&payload.provider)
        .ok_or_else(|| bad_request("unknown oauth provider in session"))?;

    // Exchange the authorization code for an access token.
    let token = provider
        .exchange_code(query.code.secret())
        .await
        .map_err(|err| map_provider_error(err, &req))?;

    // Encrypt the access token before storing it.
    let encryption = &app.config.oauth_token_encryption;
    let encrypted_token = encryption.encrypt(token.secret()).map_err(|error| {
        error!("Failed to encrypt OAuth token: {error}");
        server_error("Internal server error")
    })?;

    // Fetch the user's profile from the provider.
    let user_info = provider
        .fetch_user_info(&token)
        .await
        .map_err(|err| map_provider_error(err, &req))?;

    let mut conn = app.db_write().await?;
    let user = save_identity_to_database(
        &payload.provider,
        &user_info,
        &encrypted_token,
        &app.emails,
        &mut conn,
    )
    .await?;

    // Log in by setting a cookie and the middleware authentication.
    session.insert(SESSION_KEY_USER_ID.to_string(), user.id.to_string());

    super::user::me::get_authenticated_user(app, req).await
}

/// Map a [`ProviderError`] to a [`BoxedAppError`], logging the error details.
fn map_provider_error(err: ProviderError, req: &Parts) -> BoxedAppError {
    req.request_log().add("provider_error", format!("{err:?}"));
    match err {
        ProviderError::InvalidCode => bad_request("invalid oauth code"),
        ProviderError::Unauthorized => bad_request("oauth token was rejected"),
        ProviderError::Malformed(_) | ProviderError::Transient { .. } => {
            server_error("Error obtaining token")
        }
    }
}

/// Save a provider-agnostic [`UserInfo`] to the db.
///
/// Right now only `"github"` is handled. Adapts back to the legacy
/// `GitHubUser` shape so the existing write path keeps working.
async fn save_identity_to_database(
    provider_name: &str,
    user_info: &UserInfo,
    encrypted_token: &[u8],
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<User> {
    match provider_name {
        crate::oauth::github_provider::PROVIDER_NAME => {
            // UserInfo.account_id is String (provider-agnostic), but GitHubUser.id
            // is i32 because crates_io_github predates this trait. GitHub IDs are
            // well within i32 range today (< 200M vs i32::MAX ~2.1B). When
            // crates_io_github widens id to i64 this parse becomes a no-op change.
            let gh_id: i32 = user_info
                .account_id
                .parse()
                .map_err(|_| diesel::result::Error::NotFound)?;
            let gh_user = GitHubUser {
                id: gh_id,
                login: user_info.login.clone(),
                name: user_info.name.clone(),
                avatar_url: user_info.avatar_url.clone(),
                email: user_info.email.clone(),
            };
            save_user_to_database(&gh_user, encrypted_token, emails, conn).await
        }
        other => {
            // Tier 2 will add Bitbucket here. Unknown provider names indicate a
            // bug in registry/session pairing — return NotFound so the session
            // controller propagates a 404 rather than crashing the worker thread.
            error!(provider = other, "save_identity_to_database: no handler for provider");
            Err(diesel::result::Error::NotFound)
        }
    }
}

pub async fn save_user_to_database(
    user: &GitHubUser,
    encrypted_token: &[u8],
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<User> {
    let new_user = NewUser::builder()
        .gh_id(user.id)
        .gh_login(&user.login)
        .maybe_name(user.name.as_deref())
        .maybe_gh_avatar(user.avatar_url.as_deref())
        .gh_encrypted_token(encrypted_token)
        .build();

    match create_or_update_user(&new_user, user.email.as_deref(), emails, conn).await {
        Ok(user) => Ok(user),
        Err(error) if is_read_only_error(&error) => {
            // If we're in read only mode, we can't update their details
            // just look for an existing user
            find_user_by_gh_id(conn, user.id).await?.ok_or(error)
        }
        Err(error) => Err(error),
    }
}

/// Inserts the user into the database, or updates an existing one.
///
/// This method also inserts the email address into the `emails` table
/// and sends a confirmation email to the user.
async fn create_or_update_user(
    new_user: &NewUser<'_>,
    email: Option<&str>,
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<User> {
    conn.transaction(|conn| {
        async move {
            let user = new_user.insert_or_update(conn).await?;

            // To assist in eventually someday allowing OAuth with more than GitHub, also
            // write the GitHub info to the `oauth_github` table. Nothing currently reads
            // from this table. Only log errors but don't fail login if this writing fails.
            let new_oauth_github = NewOauthGithub::builder()
                .user_id(user.id)
                .account_id(user.gh_id as i64)
                .encrypted_token(new_user.gh_encrypted_token)
                .login(&user.gh_login)
                .maybe_avatar(user.gh_avatar.as_deref())
                .build();
            if let Err(e) = new_oauth_github.insert_or_update(conn).await {
                error!("Error inserting or updating oauth_github record: {e}");
            }

            // To send the user an account verification email
            if let Some(user_email) = email {
                let new_email = NewEmail::builder()
                    .user_id(user.id)
                    .email(user_email)
                    .build();

                if let Some(token) = new_email.insert_if_missing(conn).await? {
                    let email = EmailMessage::from_template(
                        "user_confirm",
                        context! {
                            user_name => user.gh_login,
                            domain => emails.domain,
                            token => token.expose_secret()
                        },
                    );

                    match email {
                        Ok(email) => {
                            // Swallows any error. Some users might insert an invalid email address here.
                            let _ = emails.send(user_email, email).await;
                        }
                        Err(error) => {
                            warn!("Failed to render user confirmation email template: {error}");
                        }
                    };
                }
            }

            Ok(user)
        }
        .scope_boxed()
    })
    .await
}

async fn find_user_by_gh_id(mut conn: &AsyncPgConnection, gh_id: i32) -> QueryResult<Option<User>> {
    User::query()
        .filter(users::gh_id.eq(gh_id))
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
    responses((status = 200, description = "Successful Response")),
)]
pub async fn end_session(session: SessionExtension) -> Json<bool> {
    session.remove(SESSION_KEY_USER_ID);
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

        let result = save_user_to_database(&gh_user, &[], &emails, &mut conn).await;

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {result:?}"
        );
    }
}
