#![deny(warnings)]

extern crate cargo_registry;
extern crate conduit_middleware;
extern crate civet;
extern crate git2;
extern crate env_logger;
extern crate s3;

use cargo_registry::{env, Env};
use civet::Server;
use std::env;
use std::fs::{self, File};
use std::sync::Arc;
use std::sync::mpsc::channel;

#[allow(dead_code)]
fn main() {
    env_logger::init().unwrap();
    let config: cargo_registry::Config = Default::default();

    let url = env("GIT_REPO_URL");
    let repo = match git2::Repository::open(&config.git_repo_checkout) {
        Ok(r) => r,
        Err(..) => {
            let _ = fs::remove_dir_all(&config.git_repo_checkout);
            fs::create_dir_all(&config.git_repo_checkout).unwrap();
            let mut cb = git2::RemoteCallbacks::new();
            cb.credentials(cargo_registry::git::credentials);
            let mut opts = git2::FetchOptions::new();
            opts.remote_callbacks(cb);
            git2::build::RepoBuilder::new()
                .fetch_options(opts)
                .clone(&url, &config.git_repo_checkout)
                .unwrap()
        }
    };
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "bors").unwrap();
    cfg.set_str("user.email", "bors@rust-lang.org").unwrap();

    let app = cargo_registry::App::new(&config);
    let app = cargo_registry::middleware(Arc::new(app));

    cargo_registry::categories::sync().unwrap();

    let heroku = env::var("HEROKU").is_ok();
    let port = if heroku {
        8888
    } else {
        env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8888)
    };
    let threads = if config.env == Env::Development {
        1
    } else {
        50
    };
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
