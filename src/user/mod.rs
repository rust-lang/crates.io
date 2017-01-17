use std::collections::HashMap;

use conduit::{Request, Response};
use conduit_cookie::{RequestSession};
use conduit_router::RequestParams;
use pg::GenericConnection;
use pg::rows::Row;
use rand::{thread_rng, Rng};

use {Model, Version};
use app::RequestApp;
use db::RequestTransaction;
use krate::{Crate, EncodableCrate};
use util::errors::NotFound;
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use version::EncodableVersion;
use http;

pub use self::middleware::{Middleware, RequestUser};

pub mod middleware;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub gh_login: String,
    pub gh_id: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub gh_access_token: String,
    pub api_token: String,
}

/// The serialization format for the `User` model.
#[derive(RustcDecodable, RustcEncodable)]
pub struct EncodableUser {
    pub id: i32,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    /// Queries the database for a user with a certain `gh_login` value.
    pub fn find_by_login(conn: &GenericConnection,
                         login: &str) -> CargoResult<User> {
        let stmt = try!(conn.prepare("SELECT * FROM users
                                      WHERE gh_login = $1"));
        let rows = try!(stmt.query(&[&login]));
        let row = try!(rows.iter().next().chain_error(|| {
            NotFound
        }));
        Ok(Model::from_row(&row))
    }

    /// Queries the database for a user with a certain `api_token` value.
    pub fn find_by_api_token(conn: &GenericConnection,
                             token: &str) -> CargoResult<User> {
        let stmt = try!(conn.prepare("SELECT * FROM users \
                                      WHERE api_token = $1 LIMIT 1"));
        let rows = try!(stmt.query(&[&token]));
        rows.iter().next().map(|r| Model::from_row(&r)).chain_error(|| {
            NotFound
        })
    }

    /// Updates a user or inserts a new user into the database.
    pub fn find_or_insert(conn: &GenericConnection,
                          id: i32,
                          login: &str,
                          email: Option<&str>,
                          name: Option<&str>,
                          avatar: Option<&str>,
                          access_token: &str,
                          api_token: &str) -> CargoResult<User> {
        // TODO: this is racy, but it looks like any other solution is...
        //       interesting! For now just do the racy thing which will report
        //       more errors than it needs to.

        let stmt = try!(conn.prepare("UPDATE users
                                      SET gh_access_token = $1,
                                          email = $2,
                                          name = $3,
                                          gh_avatar = $4,
                                          gh_login = $5
                                      WHERE gh_id = $6
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&access_token,
                                     &email,
                                     &name,
                                     &avatar,
                                     &login,
                                     &id]));
        match rows.iter().next() {
            Some(ref row) => return Ok(Model::from_row(row)),
            None => {}
        }
        let stmt = try!(conn.prepare("INSERT INTO users
                                      (email, gh_access_token, api_token,
                                       gh_login, name, gh_avatar, gh_id)
                                      VALUES ($1, $2, $3, $4, $5, $6, $7)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&email,
                                     &access_token,
                                     &api_token,
                                     &login,
                                     &name,
                                     &avatar,
                                     &id]));
        Ok(Model::from_row(&try!(rows.iter().next().chain_error(|| {
            internal("no user with email we just found")
        }))))
    }

    /// Generates a new crates.io API token.
    pub fn new_api_token() -> String {
        thread_rng().gen_ascii_chars().take(32).collect()
    }

    /// Converts this `User` model into an `EncodableUser` for JSON serialization.
    pub fn encodable(self) -> EncodableUser {
        let User { id, email, api_token: _, gh_access_token: _,
                   name, gh_login, avatar, gh_id: _ } = self;
        EncodableUser {
            id: id,
            email: email,
            avatar: avatar,
            login: gh_login,
            name: name,
        }
    }
}

impl Model for User {
    fn from_row(row: &Row) -> User {
        User {
            id: row.get("id"),
            email: row.get("email"),
            gh_access_token: row.get("gh_access_token"),
            api_token: row.get("api_token"),
            gh_login: row.get("gh_login"),
            gh_id: row.get("gh_id"),
            name: row.get("name"),
            avatar: row.get("gh_avatar"),
        }
    }

    fn table_name(_: Option<User>) -> &'static str { "users" }
}

/// Handles the `GET /authorize_url` route.
///
/// This route will return an authorization URL for the GitHub OAuth flow including the crates.io
/// `client_id` and a randomly generated `state` secret.
///
/// see https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access
///
/// ## Response Body Example
///
/// ```json
/// {
///     "state": "b84a63c4ea3fcb4ac84",
///     "url": "https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg"
/// }
/// ```
pub fn github_authorize(req: &mut Request) -> CargoResult<Response> {
    // Generate a random 16 char ASCII string
    let state: String = thread_rng().gen_ascii_chars().take(16).collect();
    req.session().insert("github_oauth_state".to_string(), state.clone());

    let url = req.app().github.authorize_url(state.clone());

    #[derive(RustcEncodable)]
    struct R { url: String, state: String }
    Ok(req.json(&R { url: url.to_string(), state: state }))
}

/// Handles the `GET /authorize` route.
///
/// This route is called from the GitHub API OAuth flow after the user accepted or rejected
/// the data access permissions. It will check the `state` parameter and then call the GitHub API
/// to exchange the temporary `code` for an API token. The API token is returned together with
/// the corresponding user information.
///
/// see https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site
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
pub fn github_access_token(req: &mut Request) -> CargoResult<Response> {
    // Parse the url query
    let mut query = req.query();
    let code = query.remove("code").unwrap_or(String::new());
    let state = query.remove("state").unwrap_or(String::new());

    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    {
        let session_state = req.session().remove(&"github_oauth_state".to_string());
        let session_state = session_state.as_ref().map(|a| &a[..]);
        if Some(&state[..]) != session_state {
            return Err(human("invalid state parameter"))
        }
    }

    #[derive(RustcDecodable)]
    struct GithubUser {
        email: Option<String>,
        name: Option<String>,
        login: String,
        id: i32,
        avatar_url: Option<String>,
    }

    // Fetch the access token from github using the code we just got
    let token = match req.app().github.exchange(code.clone()) {
        Ok(token) => token,
        Err(s) => return Err(human(s)),
    };

    let (handle, resp) = try!(http::github(req.app(), "/user", &token));
    let ghuser: GithubUser = try!(http::parse_github_response(handle, resp));

    // Into the database!
    let api_token = User::new_api_token();
    let user = try!(User::find_or_insert(try!(req.tx()),
                                         ghuser.id,
                                         &ghuser.login,
                                         ghuser.email.as_ref()
                                               .map(|s| &s[..]),
                                         ghuser.name.as_ref()
                                               .map(|s| &s[..]),
                                         ghuser.avatar_url.as_ref()
                                               .map(|s| &s[..]),
                                         &token.access_token,
                                         &api_token));
    req.session().insert("user_id".to_string(), user.id.to_string());
    req.mut_extensions().insert(user);
    me(req)
}

/// Handles the `GET /logout` route.
pub fn logout(req: &mut Request) -> CargoResult<Response> {
    req.session().remove(&"user_id".to_string());
    Ok(req.json(&true))
}

/// Handles the `GET /me/reset_token` route.
pub fn reset_token(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());

    let token = User::new_api_token();
    let conn = try!(req.tx());
    try!(conn.execute("UPDATE users SET api_token = $1 WHERE id = $2",
                      &[&token, &user.id]));

    #[derive(RustcEncodable)]
    struct R { api_token: String }
    Ok(req.json(&R { api_token: token }))
}

/// Handles the `GET /me` route.
pub fn me(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());

    #[derive(RustcEncodable)]
    struct R { user: EncodableUser, api_token: String }
    let token = user.api_token.clone();
    Ok(req.json(&R{ user: user.clone().encodable(), api_token: token }))
}

/// Handles the `GET /users/:user_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let name = &req.params()["user_id"];
    let conn = try!(req.tx());
    let user = try!(User::find_by_login(conn, &name));

    #[derive(RustcEncodable)]
    struct R {
        user: EncodableUser,
    }
    Ok(req.json(&R{ user: user.clone().encodable() }))
}


/// Handles the `GET /me/updates` route.
pub fn updates(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());
    let (offset, limit) = try!(req.pagination(10, 100));
    let tx = try!(req.tx());
    let sql = "SELECT versions.* FROM versions
               INNER JOIN follows
                  ON follows.user_id = $1 AND
                     follows.crate_id = versions.crate_id
               ORDER BY versions.created_at DESC OFFSET $2 LIMIT $3";

    // Load all versions
    let stmt = try!(tx.prepare(sql));
    let mut versions = Vec::new();
    let mut crate_ids = Vec::new();
    for row in try!(stmt.query(&[&user.id, &offset, &limit])).iter() {
        let version: Version = Model::from_row(&row);
        crate_ids.push(version.crate_id);
        versions.push(version);
    }

    // Load all crates
    let mut map = HashMap::new();
    let mut crates = Vec::new();
    if crate_ids.len() > 0 {
        let stmt = try!(tx.prepare("SELECT * FROM crates WHERE id = ANY($1)"));
        for row in try!(stmt.query(&[&crate_ids])).iter() {
            let krate: Crate = Model::from_row(&row);
            map.insert(krate.id, krate.name.clone());
            crates.push(krate);
        }
    }

    // Encode everything!
    let crates = crates.into_iter().map(|c| {
        c.minimal_encodable(None)
    }).collect();
    let versions = versions.into_iter().map(|v| {
        let id = v.crate_id;
        v.encodable(&map[&id])
    }).collect();

    // Check if we have another
    let sql = format!("SELECT 1 WHERE EXISTS({})", sql);
    let stmt = try!(tx.prepare(&sql));
    let more = try!(stmt.query(&[&user.id, &(offset + limit), &limit]))
                  .iter().next().is_some();

    #[derive(RustcEncodable)]
    struct R {
        versions: Vec<EncodableVersion>,
        crates: Vec<EncodableCrate>,
        meta: Meta,
    }
    #[derive(RustcEncodable)]
    struct Meta { more: bool }
    Ok(req.json(&R{ versions: versions, crates: crates, meta: Meta { more: more } }))
}
