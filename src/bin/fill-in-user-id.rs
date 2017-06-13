// Purge all references to a crate's version from the database.
//
// Please be super sure you want to do this before running this.
//
// Usage:
//      cargo run --bin delete-version crate-name version-number

#![deny(warnings)]

extern crate cargo_registry;
extern crate git2;
extern crate postgres;
extern crate rustc_serialize;

use std::path::PathBuf;

use cargo_registry::{http, env, App, Replica};
use cargo_registry::util::{CargoResult, human};

#[allow(dead_code)]
fn main() {
    git2::Repository::init("tmp/test").unwrap();
    let api_protocol = String::from("https");
    let uploader = cargo_registry::Uploader::NoOp;
    let config = cargo_registry::Config {
        uploader: uploader,
        session_key: String::new(),
        git_repo_checkout: PathBuf::from("tmp/test"),
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_registry::Env::Production,
        max_upload_size: 0,
        mirror: Replica::Primary,
        api_protocol: api_protocol,
    };
    let app = cargo_registry::App::new(&config);
    {
        let tx = app.database.get().unwrap();
        let tx = tx.transaction().unwrap();
        update(&app, &tx);
        tx.set_commit();
        tx.finish().unwrap();
    }
}

#[derive(RustcDecodable)]
struct GithubUser {
    login: String,
    id: i32,
}

fn update(app: &App, tx: &postgres::transaction::Transaction) {
    let query = "SELECT id, gh_login, gh_access_token, gh_avatar FROM users
                 WHERE gh_id IS NULL";
    let rows = tx.query(query, &[]).unwrap().into_iter().map(|row| {
        let id: i32 = row.get("id");
        let login: String = row.get("gh_login");
        let token: String = row.get("gh_access_token");
        let avatar: Option<String> = row.get("gh_avatar");
        (id, login, http::token(token), avatar)
    }).collect::<Vec<_>>();

    for (id, login, token, avatar) in rows {
        println!("attempt: {}/{}", id, login);
        let res = (|| -> CargoResult<()> {
            let url = format!("/users/{}", login);
            let (handle, resp) = http::github(app, &url, &token)?;
            let ghuser: GithubUser = http::parse_github_response(handle, &resp)?;
            if let Some(ref avatar) = avatar {
                if !avatar.contains(&ghuser.id.to_string()) {
                    return Err(human(&format_args!("avatar: {}", avatar)))
                }
            }
            if ghuser.login == login {
                tx.execute("UPDATE users SET gh_id = $1 WHERE id = $2",
                           &[&ghuser.id, &id])?;
                Ok(())
            } else {
                Err(human(&format_args!("different login: {}", ghuser.login)))
            }
        })();
        if let Err(e) = res {
            println!("error for {}: {:?}", login, e);
        }
    }
}

