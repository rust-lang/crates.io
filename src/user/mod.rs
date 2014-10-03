use std::any::AnyRefExt;
use std::rand::{task_rng, Rng};
use std::str;
use serialize::json;

use conduit::{Request, Response};
use conduit_cookie::{RequestSession};
use curl::http;
use oauth2::Authorization;
use pg::PostgresRow;
use pg::types::ToSql;

use Model;
use app::RequestApp;
use db::{Connection, RequestTransaction};
use krate::{Crate, EncodableCrate};
use util::{RequestUtils, CargoResult, internal, Require, ChainError, human};
use util::errors::NotFound;

pub use self::middleware::{Middleware, RequestUser};

mod middleware;

#[deriving(Clone, Show, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub gh_login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub gh_access_token: String,
    pub api_token: String,
}

#[deriving(Decodable, Encodable)]
pub struct EncodableUser {
    pub id: i32,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    pub fn find(conn: &Connection, id: i32) -> CargoResult<User> {
        let stmt = try!(conn.prepare("SELECT * FROM users WHERE id = $1 LIMIT 1"));
        return try!(stmt.query(&[&id])).next().map(User::from_row).require(|| {
            NotFound
        })
    }

    pub fn find_by_api_token(conn: &Connection, token: &str) -> CargoResult<User> {
        let stmt = try!(conn.prepare("SELECT * FROM users \
                                      WHERE api_token = $1 LIMIT 1"));
        return try!(stmt.query(&[&token as &ToSql])).next()
                        .map(User::from_row).require(|| {
            NotFound
        })
    }

    pub fn find_or_insert(conn: &Connection,
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
                                          gh_avatar = $4
                                      WHERE gh_login = $5
                                      RETURNING *"));
        let mut rows = try!(stmt.query(&[&access_token as &ToSql,
                                         &email, &name, &avatar,
                                         &login as &ToSql]));
        match rows.next() {
            Some(row) => return Ok(User::from_row(row)),
            None => {}
        }
        let stmt = try!(conn.prepare("INSERT INTO users
                                      (email, gh_access_token, api_token,
                                       gh_login, name, gh_avatar)
                                      VALUES ($1, $2, $3, $4, $5, $6)
                                      RETURNING *"));
        let mut rows = try!(stmt.query(&[&email as &ToSql,
                                         &access_token as &ToSql,
                                         &api_token as &ToSql,
                                         &login as &ToSql,
                                         &name, &avatar]));
        Ok(User::from_row(try!(rows.next().require(|| {
            internal("no user with email we just found")
        }))))
    }

    fn from_row(row: PostgresRow) -> User {
        User {
            id: row.get("id"),
            email: row.get("email"),
            gh_access_token: row.get("gh_access_token"),
            api_token: row.get("api_token"),
            gh_login: row.get("gh_login"),
            name: row.get("name"),
            avatar: row.get("gh_avatar"),
        }
    }

    pub fn new_api_token() -> String {
        task_rng().gen_ascii_chars().take(32).collect()
    }

    pub fn encodable(self) -> EncodableUser {
        let User { id, email, api_token: _, gh_access_token: _,
                   name, gh_login, avatar } = self;
        EncodableUser {
            id: id,
            email: email,
            avatar: avatar,
            login: gh_login,
            name: name,
        }
    }
}

pub fn github_authorize(req: &mut Request) -> CargoResult<Response> {
    let state: String = task_rng().gen_ascii_chars().take(16).collect();
    req.session().insert("github_oauth_state".to_string(), state.clone());

    let url = req.app().github.authorize_url(state.clone());

    #[deriving(Encodable)]
    struct R { url: String, state: String }
    Ok(req.json(&R { url: url.to_string(), state: state }))
}

pub fn github_access_token(req: &mut Request) -> CargoResult<Response> {
    // Parse the url query
    let mut query = req.query();
    let code = query.pop_equiv(&"code").unwrap_or(String::new());
    let state = query.pop_equiv(&"state").unwrap_or(String::new());

    // Make sure that the state we just got matches the session state that we
    // should have issued earlier.
    {
        let session_state = req.session().pop(&"github_oauth_state".to_string());
        let session_state = session_state.as_ref().map(|a| a.as_slice());
        if Some(state.as_slice()) != session_state {
            return Err(human("invalid state parameter"))
        }
    }

    // Fetch the access token from github using the code we just got
    let token = match req.app().github.exchange(code.clone()) {
        Ok(token) => token,
        Err(s) => return Err(human(s)),
    };

    let resp = try!(http::handle().get("https://api.github.com/user")
                         .header("Accept", "application/vnd.github.v3+json")
                         .header("User-Agent", "hello!")
                         .auth_with(&token)
                         .exec());
    if resp.get_code() != 200 {
        return Err(internal(format!("didn't get a 200 result from github: {}",
                                    resp)))
    }

    #[deriving(Decodable)]
    struct GithubUser {
        email: Option<String>,
        name: Option<String>,
        login: String,
        avatar_url: Option<String>,
    }
    let json = try!(str::from_utf8(resp.get_body()).require(||{
        internal("github didn't send a utf8-response")
    }));
    let ghuser: GithubUser = try!(json::decode(json).chain_error(|| {
        internal("github didn't send a valid json response")
    }));

    // Into the database!
    let api_token = User::new_api_token();
    let user = try!(User::find_or_insert(try!(req.tx()),
                                         ghuser.login.as_slice(),
                                         ghuser.email.as_ref()
                                               .map(|s| s.as_slice()),
                                         ghuser.name.as_ref()
                                               .map(|s| s.as_slice()),
                                         ghuser.avatar_url.as_ref()
                                               .map(|s| s.as_slice()),
                                         token.access_token.as_slice(),
                                         api_token.as_slice()));
    req.session().insert("user_id".to_string(), user.id.to_string());

    #[deriving(Encodable)]
    struct R { user: EncodableUser }
    Ok(req.json(&R { user: user.encodable() }))
}

pub fn logout(req: &mut Request) -> CargoResult<Response> {
    req.session().remove(&"user_id".to_string());
    Ok(req.json(&true))
}

pub fn reset_token(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());

    let token = User::new_api_token();
    let conn = try!(req.tx());
    try!(conn.execute("UPDATE users SET api_token = $1 WHERE id = $2",
                      &[&token, &user.id]));

    #[deriving(Encodable)]
    struct R { api_token: String }
    Ok(req.json(&R { api_token: token }))
}

pub fn me(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());

    #[deriving(Encodable)]
    struct R { user: EncodableUser, api_token: String }
    let token = user.api_token.clone();
    Ok(req.json(&R{ user: user.clone().encodable(), api_token: token }))
}

pub fn my_crates(req: &mut Request) -> CargoResult<Response> {
    let user = try!(req.user());
    let (offset, limit) = try!(req.pagination(10, 100));

    let tx = try!(req.tx());
    let stmt = try!(tx.prepare("SELECT * FROM crates WHERE user_id = $1
                                OFFSET $2 LIMIT $3"));
    let mut crates = Vec::new();
    for row in try!(stmt.query(&[&user.id, &offset, &limit])) {
        crates.push(Model::from_row(&row));
    }

    #[deriving(Encodable)]
    struct R { crates: Vec<EncodableCrate> }
    Ok(req.json(&R{ crates: try!(Crate::encode_many(tx, crates)) }))
}
