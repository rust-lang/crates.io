use conduit::{Request, Response};
use conduit_cookie::RequestSession;
use conduit_router::RequestParams;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use pg::GenericConnection;
use pg::rows::Row;
use rand::{thread_rng, Rng};
use std::borrow::Cow;
use serde_json;

use app::RequestApp;
use db::RequestTransaction;
use krate::Follow;
use pagination::Paginate;
use schema::*;
use util::errors::NotFound;
use util::{RequestUtils, CargoResult, internal, ChainError, human};
use version::EncodableVersion;
use {http, Model, Version};
use owner::{Owner, OwnerKind, CrateOwner};
use krate::Crate;

pub use self::middleware::{Middleware, RequestUser, AuthenticationSource};

pub mod middleware;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, PartialEq, Eq, Queryable, Identifiable, AsChangeset)]
pub struct User {
    pub id: i32,
    pub email: Option<String>,
    pub gh_access_token: String,
    pub gh_login: String,
    pub name: Option<String>,
    pub gh_avatar: Option<String>,
    pub gh_id: i32,
}

#[derive(Insertable, AsChangeset, Debug)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub email: Option<&'a str>,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_access_token: Cow<'a, str>,
}

impl<'a> NewUser<'a> {
    pub fn new(
        gh_id: i32,
        gh_login: &'a str,
        email: Option<&'a str>,
        name: Option<&'a str>,
        gh_avatar: Option<&'a str>,
        gh_access_token: &'a str,
    ) -> Self {
        NewUser {
            gh_id: gh_id,
            gh_login: gh_login,
            email: email,
            name: name,
            gh_avatar: gh_avatar,
            gh_access_token: Cow::Borrowed(gh_access_token),
        }
    }

    /// Inserts the user into the database, or updates an existing one.
    pub fn create_or_update(&self, conn: &PgConnection) -> QueryResult<User> {
        use diesel::insert;
        use diesel::expression::dsl::sql;
        use diesel::types::Integer;
        use diesel::pg::upsert::*;

        let update_user = NewUser {
            email: None,
            gh_id: self.gh_id,
            gh_login: self.gh_login,
            name: self.name,
            gh_avatar: self.gh_avatar,
            gh_access_token: self.gh_access_token.clone(),
        };

        let conflict_target = sql::<Integer>("(gh_id) WHERE gh_id > 0");
        insert(&self.on_conflict(
                conflict_target,
                do_update().set(&update_user),
            ))
            .into(users::table)
            .get_result(conn)
            .map_err(Into::into)
    }
}

/// The serialization format for the `User` model.
/// Same as private user, except no email field
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodablePublicUser {
    pub id: i32,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
}

/// The serialization format for the `User` model.
/// Same as public user, except for addition of
/// email field
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodablePrivateUser {
    pub id: i32,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
}

impl User {
    /// Queries the database for a user with a certain `gh_login` value.
    pub fn find_by_login(conn: &GenericConnection, login: &str) -> CargoResult<User> {
        let stmt = conn.prepare(
            "SELECT * FROM users
                                      WHERE gh_login = $1",
        )?;
        let rows = stmt.query(&[&login])?;
        let row = rows.iter().next().chain_error(|| NotFound)?;
        Ok(Model::from_row(&row))
    }

    /// Queries the database for a user with a certain `api_token` value.
    pub fn find_by_api_token(conn: &PgConnection, token_: &str) -> CargoResult<User> {
        use diesel::update;
        use diesel::expression::now;
        use schema::api_tokens::dsl::{api_tokens, token, user_id, last_used_at};
        use schema::users::dsl::{users, id};
        let user_id_ = update(api_tokens.filter(token.eq(token_)))
            .set(last_used_at.eq(now.nullable()))
            .returning(user_id)
            .get_result::<i32>(conn)?;
        Ok(users.filter(id.eq(user_id_)).get_result(conn)?)
    }

    /// Updates a user or inserts a new user into the database.
    pub fn find_or_insert(
        conn: &GenericConnection,
        id: i32,
        login: &str,
        email: Option<&str>,
        name: Option<&str>,
        avatar: Option<&str>,
        access_token: &str,
    ) -> CargoResult<User> {
        // TODO: this is racy, but it looks like any other solution is...
        //       interesting! For now just do the racy thing which will report
        //       more errors than it needs to.

        let stmt = conn.prepare(
            "UPDATE users
                                      SET gh_access_token = $1,
                                          email = $2,
                                          name = $3,
                                          gh_avatar = $4,
                                          gh_login = $5
                                      WHERE gh_id = $6
                                      RETURNING *",
        )?;
        let rows = stmt.query(
            &[&access_token, &email, &name, &avatar, &login, &id],
        )?;
        if let Some(ref row) = rows.iter().next() {
            return Ok(Model::from_row(row));
        }
        let stmt = conn.prepare(
            "INSERT INTO users
                                      (email, gh_access_token,
                                       gh_login, name, gh_avatar, gh_id)
                                      VALUES ($1, $2, $3, $4, $5, $6)
                                      RETURNING *",
        )?;
        let rows = stmt.query(
            &[&email, &access_token, &login, &name, &avatar, &id],
        )?;
        Ok(Model::from_row(&rows.iter().next().chain_error(|| {
            internal("no user with email we just found")
        })?))
    }

    pub fn owning(krate: &Crate, conn: &PgConnection) -> CargoResult<Vec<Owner>> {
        let base_query = CrateOwner::belonging_to(krate).filter(crate_owners::deleted.eq(false));
        let users = base_query
            .inner_join(users::table)
            .select(users::all_columns)
            .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32))
            .load(conn)?
            .into_iter()
            .map(Owner::User);

        Ok(users.collect())
    }

    /// Converts this `User` model into an `EncodablePrivateUser` for JSON serialization.
    pub fn encodable_private(self) -> EncodablePrivateUser {
        let User {
            id,
            email,
            name,
            gh_login,
            gh_avatar,
            ..
        } = self;
        let url = format!("https://github.com/{}", gh_login);
        EncodablePrivateUser {
            id: id,
            email: email,
            avatar: gh_avatar,
            login: gh_login,
            name: name,
            url: Some(url),
        }
    }

    /// Converts this`User` model into an `EncodablePublicUser` for JSON serialization.
    pub fn encodable_public(self) -> EncodablePublicUser {
        let User {
            id,
            name,
            gh_login,
            gh_avatar,
            ..
        } = self;
        let url = format!("https://github.com/{}", gh_login);
        EncodablePublicUser {
            id: id,
            avatar: gh_avatar,
            login: gh_login,
            name: name,
            url: Some(url),
        }
    }
}

impl Model for User {
    fn from_row(row: &Row) -> User {
        User {
            id: row.get("id"),
            email: row.get("email"),
            gh_access_token: row.get("gh_access_token"),
            gh_login: row.get("gh_login"),
            gh_id: row.get("gh_id"),
            name: row.get("name"),
            gh_avatar: row.get("gh_avatar"),
        }
    }

    fn table_name(_: Option<User>) -> &'static str {
        "users"
    }
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
    req.session().insert(
        "github_oauth_state".to_string(),
        state.clone(),
    );

    let url = req.app().github.authorize_url(state.clone());

    #[derive(Serialize)]
    struct R {
        url: String,
        state: String,
    }
    Ok(req.json(&R {
        url: url.to_string(),
        state: state,
    }))
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

    #[derive(Deserialize)]
    struct GithubUser {
        email: Option<String>,
        name: Option<String>,
        login: String,
        id: i32,
        avatar_url: Option<String>,
    }

    // Fetch the access token from github using the code we just got
    let token = req.app().github.exchange(code.clone()).map_err(
        |s| human(&s),
    )?;

    let (handle, resp) = http::github(req.app(), "/user", &token)?;
    let ghuser: GithubUser = http::parse_github_response(handle, &resp)?;

    let user = NewUser::new(
        ghuser.id,
        &ghuser.login,
        ghuser.email.as_ref().map(|s| &s[..]),
        ghuser.name.as_ref().map(|s| &s[..]),
        ghuser.avatar_url.as_ref().map(|s| &s[..]),
        &token.access_token,
    ).create_or_update(&*req.db_conn()?)?;
    req.session().insert(
        "user_id".to_string(),
        user.id.to_string(),
    );
    req.mut_extensions().insert(user);
    me(req)
}

/// Handles the `GET /logout` route.
pub fn logout(req: &mut Request) -> CargoResult<Response> {
    req.session().remove(&"user_id".to_string());
    Ok(req.json(&true))
}

/// Handles the `GET /me` route.
pub fn me(req: &mut Request) -> CargoResult<Response> {
    // Changed to getting User information from database because in
    // src/tests/user.rs, when testing put and get on updating email,
    // request seems to be somehow 'cached'. When we try to get a
    // request from the /me route with the just updated user (call
    // this function) the user is the same as the initial GET request
    // and does not seem to get the updated user information from the
    // database
    // This change is not preferable, we'd rather fix the request,
    // perhaps adding `req.mut_extensions().insert(user)` to the
    // update_user route, however this somehow does not seem to work
    use self::users::dsl::{users, id};
    let user_id = req.user()?.id;
    let conn = req.db_conn()?;
    let user = users.filter(id.eq(user_id)).first::<User>(&*conn)?;
    println!("user id: {:?} user_id: {:?}", user.id, user_id);

    #[derive(Serialize)]
    struct R {
        user: EncodablePrivateUser,
    }
    Ok(req.json(&R { user: user.encodable_private() }))
}

/// Handles the `GET /users/:user_id` route.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    use self::users::dsl::{users, gh_login};

    let name = &req.params()["user_id"];
    let conn = req.db_conn()?;
    let user = users.filter(gh_login.eq(name)).first::<User>(&*conn)?;

    #[derive(Serialize)]
    struct R {
        user: EncodablePublicUser,
    }
    Ok(req.json(&R { user: user.encodable_public() }))
}

/// Handles the `GET /teams/:team_id` route.
pub fn show_team(req: &mut Request) -> CargoResult<Response> {
    use self::teams::dsl::{teams, login};
    use owner::Team;
    use owner::EncodableTeam;

    let name = &req.params()["team_id"];
    let conn = req.db_conn()?;
    let team = teams.filter(login.eq(name)).first::<Team>(&*conn)?;

    #[derive(Serialize)]
    struct R {
        team: EncodableTeam,
    }
    Ok(req.json(&R { team: team.encodable() }))
}

/// Handles the `GET /me/updates` route.
pub fn updates(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::any;

    let user = req.user()?;
    let (offset, limit) = req.pagination(10, 100)?;
    let conn = req.db_conn()?;

    let followed_crates = Follow::belonging_to(user).select(follows::crate_id);
    let data = versions::table
        .inner_join(crates::table)
        .filter(crates::id.eq(any(followed_crates)))
        .order(versions::created_at.desc())
        .select((versions::all_columns, crates::name))
        .paginate(limit, offset)
        .load::<((Version, String), i64)>(&*conn)?;

    let more = data.get(0)
        .map(|&(_, count)| count > offset + limit)
        .unwrap_or(false);

    let versions = data.into_iter()
        .map(|((version, crate_name), _)| version.encodable(&crate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        more: bool,
    }
    Ok(req.json(&R {
        versions: versions,
        meta: Meta { more: more },
    }))
}

/// Handles the `GET /users/:user_id/stats` route.
pub fn stats(req: &mut Request) -> CargoResult<Response> {
    use diesel::expression::dsl::sum;
    use owner::OwnerKind;

    let user_id = &req.params()["user_id"].parse::<i32>().ok().unwrap();
    let conn = req.db_conn()?;

    let data = crate_owners::table
        .inner_join(crates::table)
        .filter(crate_owners::owner_id.eq(user_id).and(
            crate_owners::owner_kind.eq(OwnerKind::User as i32),
        ))
        .select(sum(crates::downloads))
        .first::<Option<i64>>(&*conn)?
        .unwrap_or(0);

    #[derive(Serialize)]
    struct R {
        total_downloads: i64,
    }
    Ok(req.json(&R { total_downloads: data }))
}

/// Handles the `PUT /user/:user_id` route.
pub fn update_user(req: &mut Request) -> CargoResult<Response> {
    use diesel::update;
    use self::users::dsl::{users, gh_login, email};

    let mut body = String::new();
    req.body().read_to_string(&mut body)?;
    let user = req.user()?;
    let name = &req.params()["user_id"];
    let conn = req.db_conn()?;

    // need to check if current user matches user to be updated
    if &user.id.to_string() != name {
        return Err(human("current user does not match requested user"));
    }

    #[derive(Deserialize)]
    struct UserUpdate {
        user: User,
    }

    #[derive(Deserialize)]
    struct User {
        email: Option<String>,
    }

    let user_update: UserUpdate = serde_json::from_str(&body).map_err(
        |_| human("invalid json request"),
    )?;

    if user_update.user.email.is_none() {
        return Err(human("empty email rejected"));
    }

    let user_email = user_update.user.email.unwrap();
    let user_email = user_email.trim();

    println!("update_user email: {:?}", user_email);

    update(users.filter(gh_login.eq(&user.gh_login)))
        .set(email.eq(user_email))
        .execute(&*conn)?;

    #[derive(Serialize)]
    struct R {
        ok: bool,
    }
    Ok(req.json(&R { ok: true }))
}
