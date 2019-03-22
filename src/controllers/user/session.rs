use crate::controllers::prelude::*;

use crate::github;
use conduit_cookie::RequestSession;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::models::{NewUser, User};
use crate::schema::users;
use crate::util::errors::{CargoError, ReadOnlyMode};

/// Handles the `GET /authorize_url` route.
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
pub fn github_authorize(req: &mut dyn Request) -> CargoResult<Response> {
    // Generate a random 16 char ASCII string
    let mut rng = thread_rng();
    let state: String = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();
    req.session()
        .insert("github_oauth_state".to_string(), state.clone());

    let url = req.app().github.authorize_url(state.clone());

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

/// Handles the `GET /authorize` route.
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
pub fn github_access_token(req: &mut dyn Request) -> CargoResult<Response> {
    // Parse the url query
    let mut query = req.query();
    let code = query.remove("code").unwrap_or_default();
    let state = query.remove("state").unwrap_or_default();

    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    {
        let session_state = req.session().remove(&"github_oauth_state".to_string());
        let session_state = session_state.as_ref().map(|a| &a[..]);
        if Some(&state[..]) != session_state {
            return Err(human("invalid state parameter"));
        }
    }

    // Fetch the access token from github using the code we just got
    let token = req.app().github.exchange(code).map_err(|s| human(&s))?;

    let ghuser = github::github::<GithubUser>(req.app(), "/user", &token)?;
    let user = ghuser.save_to_database(&token.access_token, &*req.db_conn()?)?;

    req.session()
        .insert("user_id".to_string(), user.id.to_string());
    req.mut_extensions().insert(user);
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
    fn save_to_database(&self, access_token: &str, conn: &PgConnection) -> CargoResult<User> {
        NewUser::new(
            self.id,
            &self.login,
            self.email.as_ref().map(|s| &s[..]),
            self.name.as_ref().map(|s| &s[..]),
            self.avatar_url.as_ref().map(|s| &s[..]),
            access_token,
        )
        .create_or_update(conn)
        .map_err(Into::into)
        .or_else(|e: Box<dyn CargoError>| {
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

/// Handles the `GET /logout` route.
pub fn logout(req: &mut dyn Request) -> CargoResult<Response> {
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
