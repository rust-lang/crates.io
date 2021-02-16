use crate::controllers::frontend_prelude::*;

use conduit_cookie::{RequestCookies, RequestSession};
use oauth2::reqwest::http_client;
use oauth2::{AuthorizationCode, Scope, TokenResponse};

use crate::controllers::util::{auth_cookie, AUTH_COOKIE_NAME};
use crate::github::GithubUser;
use crate::models::{NewUser, Session, User};
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

    #[derive(Serialize)]
    struct R {
        url: String,
        state: String,
    }
    Ok(req.json(&R {
        url: url.to_string(),
        state,
    }))
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
        .chain_error(|| server_error("Error obtaining token"))?;
    let token = token.access_token();

    // Fetch the user info from GitHub using the access token we just got and create a user record
    let ghuser = req.app().github.current_user(token)?;
    let user = save_user_to_database(&ghuser, &token.secret(), &*req.db_conn()?)?;

    // Create a new `Session`
    let token = Session::generate_token();

    let ip_addr = req.remote_addr().ip();

    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .unwrap_or_default();

    Session::new()
        .user_id(user.id)
        .token(&token)
        .last_ip_address(ip_addr)
        .last_user_agent(user_agent)
        .build()
        .map_err(|_err| server_error("Error obtaining token"))?
        .insert(&*req.db_conn()?)?;

    // Log in by setting an auth cookie
    let app = req.app();
    let secure = app.config.env == Env::Production;

    req.cookies_mut().add(auth_cookie(token, secure));

    super::me::me(req)
}

fn save_user_to_database(
    user: &GithubUser,
    access_token: &str,
    conn: &PgConnection,
) -> AppResult<User> {
    NewUser::new(
        user.id,
        &user.login,
        user.name.as_deref(),
        user.avatar_url.as_deref(),
        access_token,
    )
    .create_or_update(user.email.as_deref(), conn)
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

/// Handles the `DELETE /api/private/session` route.
pub fn logout(req: &mut dyn RequestExt) -> EndpointResult {
    req.session_mut().remove(&"user_id".to_string());

    // read the current session token, if it exists
    let session_token = req
        .cookies()
        .get(AUTH_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string());

    if let Some(token) = session_token {
        let app = req.app();
        let secure = app.config.env == Env::Production;

        // remove the `cargo_auth` cookie
        req.cookies_mut().remove(auth_cookie("", secure));

        // try to revoke the session in the database, but explicitly
        // ignore failures
        let _result: Result<_, Box<dyn AppError>> = req
            .db_conn()
            .map_err(Into::into)
            .and_then(|conn| Session::revoke_by_token(&conn, &token).map_err(Into::into));
    }

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
        let conn = pg_connection();
        let gh_user = GithubUser {
            email: Some("String.Format(\"{0}.{1}@live.com\", FirstName, LastName)".into()),
            name: Some("My Name".into()),
            login: "github_user".into(),
            id: -1,
            avatar_url: None,
        };
        let result = save_user_to_database(&gh_user, "arbitrary_token", &conn);

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {:?}",
            result
        );
    }
}
