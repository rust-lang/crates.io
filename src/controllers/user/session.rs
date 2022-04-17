use crate::controllers::frontend_prelude::*;

use conduit_cookie::{RequestCookies, RequestSession};
use cookie::Cookie;
use oauth2::reqwest::http_client;
use oauth2::{AuthorizationCode, Scope, TokenResponse};
use thiserror::Error;

use crate::email::Emails;
use crate::github::GithubUser;
use crate::models::persistent_session::ParseSessionCookieError;
use crate::models::persistent_session::SessionCookie;
use crate::models::{NewUser, PersistentSession, User};
use crate::schema::users;
use crate::util::errors::ReadOnlyMode;
use crate::Env;

/// Handles the `GET /api/private/session/begin` route.
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
pub fn begin(req: &mut dyn RequestExt) -> EndpointResult {
    let (url, state) = req
        .app()
        .github_oauth
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(Scope::new("read:org".to_string()))
        .url();
    let state = state.secret().to_string();
    req.session_mut()
        .insert("github_oauth_state".to_string(), state.clone());

    Ok(req.json(&json!({ "url": url.to_string(), "state": state })))
}

/// Handles the `GET /api/private/session/authorize` route.
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
///     "api_token": "b84a63c4ea3fcb4ac84",
///     "user": {
///         "email": "foo@bar.org",
///         "name": "Foo Bar",
///         "login": "foobar",
///         "avatar": "https://avatars.githubusercontent.com/u/1234",
///         "url": null
///     }
/// }
/// ```
pub fn authorize(req: &mut dyn RequestExt) -> EndpointResult {
    // Parse the url query
    let mut query = req.query();
    let code = query.remove("code").unwrap_or_default();
    let state = query.remove("state").unwrap_or_default();

    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    {
        let session_state = req.session_mut().remove(&"github_oauth_state".to_string());
        let session_state = session_state.as_deref();
        if Some(&state[..]) != session_state {
            return Err(bad_request("invalid state parameter"));
        }
    }

    // Fetch the access token from GitHub using the code we just got
    let code = AuthorizationCode::new(code);
    let token = req
        .app()
        .github_oauth
        .exchange_code(code)
        .request(http_client)
        .map_err(|err| err.chain(server_error("Error obtaining token")))?;
    let token = token.access_token();

    // Fetch the user info from GitHub using the access token we just got and create a user record
    let ghuser = req.app().github.current_user(token)?;
    let user = save_user_to_database(
        &ghuser,
        token.secret(),
        &req.app().emails,
        &*req.db_write()?,
    )?;

    // Setup a persistent session for the newly logged in user.
    let (_session, cookie) = PersistentSession::create(user.id).insert(&*req.db_conn()?)?;

    // Setup persistent session cookie.
    let secure = req.app().config.env() == Env::Production;
    req.cookies_mut().add(cookie.build(secure));

    // TODO(adsnaider): Remove as part of https://github.com/rust-lang/crates.io/issues/2630.
    // Log in by setting a cookie and the middleware authentication.
    req.session_mut()
        .insert("user_id".to_string(), user.id.to_string());

    super::me::me(req)
}

fn save_user_to_database(
    user: &GithubUser,
    access_token: &str,
    emails: &Emails,
    conn: &PgConnection,
) -> AppResult<User> {
    NewUser::new(
        user.id,
        &user.login,
        user.name.as_deref(),
        user.avatar_url.as_deref(),
        access_token,
    )
    .create_or_update(user.email.as_deref(), emails, conn)
    .map_err(Into::into)
    .or_else(|e: Box<dyn AppError>| {
        // If we're in read only mode, we can't update their details
        // just look for an existing user
        if e.is::<ReadOnlyMode>() {
            users::table
                .filter(users::gh_id.eq(user.id))
                .first(conn)
                .optional()?
                .ok_or(e)
        } else {
            Err(e)
        }
    })
}

#[derive(Error, Debug, PartialEq)]
pub enum LogoutError {
    #[error("No session cookie found.")]
    MissingSessionCookie,
    #[error("Session cookie had an unexpected format.")]
    SessionCookieMalformatted(#[from] ParseSessionCookieError),
    #[error("Session is not in the database.")]
    SessionNotInDB,
}

/// Handles the `DELETE /api/private/session` route.
pub fn logout(req: &mut dyn RequestExt) -> EndpointResult {
    // TODO(adsnaider): Remove as part of https://github.com/rust-lang/crates.io/issues/2630.
    req.session_mut().remove(&"user_id".to_string());

    // Remove persistent session from database.
    let session_cookie = req
        .cookies()
        .get(SessionCookie::SESSION_COOKIE_NAME)
        .ok_or(LogoutError::MissingSessionCookie)?
        .value()
        .parse::<SessionCookie>()?;

    req.cookies_mut()
        .remove(Cookie::named(SessionCookie::SESSION_COOKIE_NAME));

    let conn = req.db_conn()?;
    let mut session = PersistentSession::find(session_cookie.session_id(), &conn)?
        .ok_or(LogoutError::SessionNotInDB)?;
    session.revoke().update(&conn)?;
    Ok(req.json(&true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pg_connection() -> PgConnection {
        let database_url =
            dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
        PgConnection::establish(&database_url).unwrap()
    }

    #[test]
    fn gh_user_with_invalid_email_doesnt_fail() {
        let emails = Emails::new_in_memory();
        let conn = pg_connection();
        let gh_user = GithubUser {
            email: Some("String.Format(\"{0}.{1}@live.com\", FirstName, LastName)".into()),
            name: Some("My Name".into()),
            login: "github_user".into(),
            id: -1,
            avatar_url: None,
        };
        let result = save_user_to_database(&gh_user, "arbitrary_token", &emails, &conn);

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {:?}",
            result
        );
    }
}
