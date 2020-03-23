use crate::controllers::frontend_prelude::*;

use crate::github;
use conduit_cookie::RequestSession;
use failure::Fail;
use oauth2::{prelude::*, AuthorizationCode, TokenResponse};

use crate::middleware::current_user::TrustedUserId;
use crate::models::{NewUser, User};
use crate::schema::users;
use crate::util::errors::ReadOnlyMode;

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
pub fn begin(req: &mut dyn Request) -> AppResult<Response> {
    let (url, state) = req
        .app()
        .github
        .authorize_url(oauth2::CsrfToken::new_random);
    let state = state.secret().to_string();
    req.session()
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
pub fn authorize(req: &mut dyn Request) -> AppResult<Response> {
    // Parse the url query
    let mut query = req.query();
    let code = query.remove("code").unwrap_or_default();
    let state = query.remove("state").unwrap_or_default();

    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    {
        let session_state = req.session().remove(&"github_oauth_state".to_string());
        let session_state = session_state.as_deref();
        if Some(&state[..]) != session_state {
            return Err(bad_request("invalid state parameter"));
        }
    }

    // Fetch the access token from GitHub using the code we just got
    let code = AuthorizationCode::new(code);
    let token = req
        .app()
        .github
        .exchange_code(code)
        .map_err(|e| e.compat())
        .chain_error(|| server_error("Error obtaining token"))?;
    let token = token.access_token();

    // Fetch the user info from GitHub using the access token we just got and create a user record
    let ghuser = github::github_api::<GithubUser>(req.app(), "/user", token)?;
    let user = ghuser.save_to_database(&token.secret(), &*req.db_conn()?)?;

    // Log in by setting a cookie and the middleware authentication
    req.session()
        .insert("user_id".to_string(), user.id.to_string());
    req.mut_extensions().insert(TrustedUserId(user.id));

    super::me::me(req)
}

#[derive(Deserialize)]
struct GithubUser {
    email: Option<String>,
    name: Option<String>,
    login: String,
    id: i32,
    avatar_url: Option<String>,
}

impl GithubUser {
    fn save_to_database(&self, access_token: &str, conn: &PgConnection) -> AppResult<User> {
        NewUser::new(
            self.id,
            &self.login,
            self.name.as_deref(),
            self.avatar_url.as_deref(),
            access_token,
        )
        .create_or_update(self.email.as_deref(), conn)
        .map_err(Into::into)
        .or_else(|e: Box<dyn AppError>| {
            // If we're in read only mode, we can't update their details
            // just look for an existing user
            if e.is::<ReadOnlyMode>() {
                users::table
                    .filter(users::gh_id.eq(self.id))
                    .first(conn)
                    .optional()?
                    .ok_or(e)
            } else {
                Err(e)
            }
        })
    }
}

/// Handles the `DELETE /api/private/session` route.
pub fn logout(req: &mut dyn Request) -> AppResult<Response> {
    req.session().remove(&"user_id".to_string());
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
        let result = gh_user.save_to_database("arbitrary_token", &conn);

        assert!(
            result.is_ok(),
            "Creating a User from a GitHub user failed when it shouldn't have, {:?}",
            result
        );
    }
}
