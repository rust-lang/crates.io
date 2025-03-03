use axum::Json;
use axum::extract::{FromRequestParts, Query};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};

use crate::app::AppState;
use crate::controllers::user::update::UserConfirmEmail;
use crate::email::Emails;
use crate::middleware::log_request::RequestLogExt;
use crate::models::{NewEmail, NewUser, User};
use crate::schema::users;
use crate::util::diesel::is_read_only_error;
use crate::util::errors::{AppResult, bad_request, server_error};
use crate::views::EncodableMe;
use crates_io_github::GithubUser;
use crates_io_session::SessionExtension;

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
    get,
    path = "/api/private/session/begin",
    tag = "session",
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
#[utoipa::path(
    get,
    path = "/api/private/session/authorize",
    tag = "session",
    responses((status = 200, description = "Successful Response", body = inline(EncodableMe))),
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
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let token = app
        .github_oauth
        .exchange_code(query.code)
        .request_async(&client)
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

    super::user::me::get_authenticated_user(app, req).await
}

pub async fn save_user_to_database(
    user: &GithubUser,
    access_token: &str,
    emails: &Emails,
    conn: &mut AsyncPgConnection,
) -> QueryResult<User> {
    let new_user = NewUser::builder()
        .gh_id(user.id)
        .gh_login(&user.login)
        .maybe_name(user.name.as_deref())
        .maybe_gh_avatar(user.avatar_url.as_deref())
        .gh_access_token(access_token)
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

            // To send the user an account verification email
            if let Some(user_email) = email {
                let new_email = NewEmail::builder()
                    .user_id(user.id)
                    .email(user_email)
                    .build();

                if let Some(token) = new_email.insert_if_missing(conn).await? {
                    // Swallows any error. Some users might insert an invalid email address here.
                    let email = UserConfirmEmail {
                        user_name: &user.gh_login,
                        domain: &emails.domain,
                        token,
                    };
                    let _ = emails.send(user_email, email).await;
                }
            }

            Ok(user)
        }
        .scope_boxed()
    })
    .await
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
    security(("cookie" = [])),
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
