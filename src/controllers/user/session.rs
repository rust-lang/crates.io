use axum::extract::{FromRequestParts, Query};
use axum::Json;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};

use crate::app::AppState;
use crate::email::Emails;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::session::SessionExtension;
use crate::models::{NewUser, User};
use crate::schema::users;
use crate::util::diesel::is_read_only_error;
use crate::util::errors::{bad_request, server_error, AppResult};
use crate::views::EncodableMe;
use crates_io_github::GithubUser;

/// Begin authentication flow.
///
/// This route will return an authorization URL for the GitHub OAuth flow including the crates.io
/// `client_id` and a randomly generated `state` secret.
///
/// see <https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access>
///
/// ## Response Body Example
///
/// ```json
/// {
///     "state": "b84a63c4ea3fcb4ac84",
///     "url": "https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg"
/// }
/// ```
#[utoipa::path(
    get,
    path = "/api/private/session/begin",
    tag = "session",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn begin_session(app: AppState, session: SessionExtension) -> ErasedJson {
    let (url, state) = app
        .github_oauth
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(Scope::new("read:org".to_string()))
        .url();

    let state = state.secret().to_string();
    session.insert("github_oauth_state".to_string(), state.clone());

    json!({ "url": url.to_string(), "state": state })
}

#[derive(Clone, Debug, Deserialize, FromRequestParts)]
#[from_request(via(Query))]
pub struct AuthorizeQuery {
    code: AuthorizationCode,
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
///
/// ## Query Parameters
///
/// - `code` – temporary code received from the GitHub API  **(Required)**
/// - `state` – state parameter received from the GitHub API  **(Required)**
///
/// ## Response Body Example
///
/// ```json
/// {
///     "user": {
///         "email": "foo@bar.org",
///         "name": "Foo Bar",
///         "login": "foobar",
///         "avatar": "https://avatars.githubusercontent.com/u/1234",
///         "url": null
///     }
/// }
/// ```
#[utoipa::path(
    get,
    path = "/api/private/session/authorize",
    tag = "session",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn authorize_session(
    query: AuthorizeQuery,
    app: AppState,
    session: SessionExtension,
    req: Parts,
) -> AppResult<Json<EncodableMe>> {
    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    let session_state = session.remove("github_oauth_state").map(CsrfToken::new);
    if session_state.is_none_or(|state| query.state.secret() != state.secret()) {
        return Err(bad_request("invalid state parameter"));
    }

    // Fetch the access token from GitHub using the code we just got
    let token = app
        .github_oauth
        .exchange_code(query.code)
        .request_async(async_http_client)
        .await
        .map_err(|err| {
            req.request_log().add("cause", err);
            server_error("Error obtaining token")
        })?;

    let token = token.access_token();

    // Fetch the user info from GitHub using the access token we just got and create a user record
    let ghuser = app.github.current_user(token).await?;

    let mut conn = app.db_write().await?;
    let user = save_user_to_database(&ghuser, token.secret(), &app.emails, &mut conn).await?;

    // Log in by setting a cookie and the middleware authentication
    session.insert("user_id".to_string(), user.id.to_string());

    super::me::get_authenticated_user(app, req).await
}

async fn save_user_to_database(
    user: &GithubUser,
    access_token: &str,
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> AppResult<User> {
    let new_user = NewUser::new(
        user.id,
        &user.login,
        user.name.as_deref(),
        user.avatar_url.as_deref(),
        access_token,
    );

    match new_user
        .create_or_update(user.email.as_deref(), emails, conn)
        .await
    {
        Ok(user) => Ok(user),
        Err(error) if is_read_only_error(&error) => {
            // If we're in read only mode, we can't update their details
            // just look for an existing user
            find_user_by_gh_id(conn, user.id)
                .await?
                .ok_or_else(|| error.into())
        }
        Err(error) => Err(error.into()),
    }
}

async fn find_user_by_gh_id(conn: &mut AsyncPgConnection, gh_id: i32) -> QueryResult<Option<User>> {
    users::table
        .filter(users::gh_id.eq(gh_id))
        .first(conn)
        .await
        .optional()
}

/// End the current session.
#[utoipa::path(
    delete,
    path = "/api/private/session",
    tag = "session",
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

        let gh_user = GithubUser {
            email: Some("String.Format(\"{0}.{1}@live.com\", FirstName, LastName)".into()),
            name: Some("My Name".into()),
            login: "github_user".into(),
            id: -1,
            avatar_url: None,
        };
        let result = save_user_to_database(&gh_user, "arbitrary_token", &emails, &mut conn).await;

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {result:?}"
        );
    }
}
