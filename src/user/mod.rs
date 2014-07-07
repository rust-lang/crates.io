use std::any::AnyRefExt;
use std::io::IoResult;
use std::rand::{task_rng, Rng};
use std::str;
use serialize::json;

use conduit::{Request, Response};
use conduit_cookie::{RequestSession};
use curl::http;
use oauth2::Authorization;
use pg::PostgresConnection;
use pg::error::PgDbError;

use app::{App, RequestApp};
use util::{RequestJson, RequestQuery};

pub use self::middleware::{Middleware, RequestUser};

mod middleware;

pub struct User {
    pub id: i32,
    pub email: String,
    pub gh_access_token: String,
}

impl User {
    pub fn find(app: &App, id: i32) -> Option<User> {
        let conn = app.db();
        let stmt = conn.prepare("SELECT * FROM users WHERE id = $1 LIMIT 1")
                       .unwrap();
        stmt.query([&id]).unwrap().next().map(|row| {
            User {
                id: row["id"],
                email: row["email"],
                gh_access_token: row["gh_access_token"]
            }
        })
    }
}

pub fn setup(conn: &PostgresConnection) {
    conn.execute("DROP TABLE IF EXISTS users", []).unwrap();
    conn.execute("CREATE TABLE users (
                    id              SERIAL PRIMARY KEY,
                    email           VARCHAR NOT NULL,
                    gh_access_token VARCHAR NOT NULL
                  )", []).unwrap();
    conn.execute("ALTER TABLE users ADD CONSTRAINT \
                  unique_email UNIQUE (email)", []).unwrap();
}

pub fn github_authorize(req: &mut Request) -> IoResult<Response> {
    let state: String = task_rng().gen_ascii_chars().take(16).collect();
    req.session().insert("github_oauth_state".to_string(), state.clone());

    let url = req.app().github.authorize_url(state);
    Ok(req.json(&url.to_str()))
}

pub fn github_access_token(req: &mut Request) -> IoResult<Response> {
    #[deriving(Encodable)]
    struct R { ok: bool, error: Option<String> }

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
            return Ok(req.json(&R {
                ok: false,
                error: Some(format!("invalid state parameter"))
            }))
        }
    }

    // Fetch the access token from github using the code we just got
    let token = match req.app().github.exchange(code.clone()) {
        Ok(token) => token,
        Err(s) => return Ok(req.json(&R { ok: false, error: Some(s) }))
    };

    // TODO: none of this should be fallible
    let resp = http::handle().get("https://api.github.com/user")
                    .header("Accept", "application/vnd.github.v3+json")
                    .header("User-Agent", "hello!")
                    .auth_with(&token)
                    .exec().unwrap();
    assert_eq!(resp.get_code(), 200);

    // TODO: more fallibility
    #[deriving(Decodable)]
    struct GithubUser { email: String }
    let json = str::from_utf8(resp.get_body()).expect("non-utf8 body");
    let ghuser: GithubUser = json::decode(json).unwrap();

    // Into the database!
    let conn = req.app().db();
    let resp = conn.execute("INSERT INTO users (email, gh_access_token) \
                             VALUES ($1, $2)",
                            [&ghuser.email.as_slice(),
                             &token.access_token.as_slice()]);
    match resp {
        Ok(..) => {}
        Err(PgDbError(ref e))
            if e.constraint.as_ref().map(|a| a.as_slice())
                == Some("unique_email") => {}
        Err(e) => fail!("postgres error: {}", e),
    }

    // Who did we just insert?
    let stmt = conn.prepare("SELECT id FROM users WHERE email = $1 LIMIT 1")
                   .unwrap();
    let row = stmt.query([&ghuser.email.as_slice()]).unwrap()
                  .next().expect("no user with email we just found");
    let id: i32 = row["id"];
    req.session().insert("user_id".to_string(), id.to_str());

    Ok(req.json(&R { ok: true, error: None }))
}

pub fn logout(req: &mut Request) -> IoResult<Response> {
    req.session().remove(&"user_id".to_string());
    Ok(req.json(&true))
}
