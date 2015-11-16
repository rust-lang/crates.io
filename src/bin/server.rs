#![deny(warnings)]

extern crate cargo_registry;
extern crate conduit_middleware;
extern crate civet;
extern crate git2;
extern crate env_logger;

use civet::Server;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;

#[allow(dead_code)]
fn main() {
    env_logger::init().unwrap();
    let url = env("GIT_REPO_URL");
    let checkout = PathBuf::from(env("GIT_REPO_CHECKOUT"));

    let repo = match git2::Repository::open(&checkout) {
        Ok(r) => r,
        Err(..) => {
            let _ = fs::remove_dir_all(&checkout);
            fs::create_dir_all(&checkout).unwrap();
            let mut cb = git2::RemoteCallbacks::new();
            cb.credentials(cargo_registry::git::credentials);
            let mut opts = git2::FetchOptions::new();
            opts.remote_callbacks(cb);
            git2::build::RepoBuilder::new()
                                     .fetch_options(opts)
                                     .clone(&url, &checkout).unwrap()
        }
    };
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "bors").unwrap();
    cfg.set_str("user.email", "bors@rust-lang.org").unwrap();

    let heroku = env::var("HEROKU").is_ok();
    let cargo_env = if heroku {
        cargo_registry::Env::Production
    } else {
        cargo_registry::Env::Development
    };
    let config = cargo_registry::Config {
        s3_bucket: env("S3_BUCKET"),
        s3_access_key: env("S3_ACCESS_KEY"),
        s3_secret_key: env("S3_SECRET_KEY"),
        s3_region: env::var("S3_REGION").ok(),
        s3_proxy: None,
        session_key: env("SESSION_KEY"),
        git_repo_checkout: checkout,
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_env,
        max_upload_size: 10 * 1024 * 1024,
    };
    let app = cargo_registry::App::new(&config);
    let app = cargo_registry::middleware(Arc::new(app));

    let port = if heroku {
        8888
    } else {
        env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8888)
    };
    let threads = if cargo_env == cargo_registry::Env::Development {1} else {50};
    let mut cfg = civet::Config::new();
    cfg.port(port).threads(threads).keep_alive(true);
    let _a = Server::start(cfg, app);
    println!("listening on port {}", port);
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    // TODO: handle a graceful shutdown by just waiting for a SIG{INT,TERM}
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}

fn env(s: &str) -> String {
    match env::var(s).ok() {
        Some(s) => s,
        None => panic!("must have `{}` defined", s),
    }
}
