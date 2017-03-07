#![deny(warnings)]

extern crate cargo_registry;
extern crate conduit_middleware;
extern crate civet;
extern crate git2;
extern crate env_logger;
extern crate s3;

use cargo_registry::env;
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

    let api_protocol = String::from("https");
    let mirror = env::var("MIRROR").is_ok();

    let heroku = env::var("HEROKU").is_ok();
    let cargo_env = if heroku {
        cargo_registry::Env::Production
    } else {
        cargo_registry::Env::Development
    };

    let uploader = match (cargo_env, mirror) {
        (cargo_registry::Env::Production, false) => {
            // `env` panics if these vars are not set
            cargo_registry::Uploader::S3 {
                bucket: s3::Bucket::new(env("S3_BUCKET"),
                                        env::var("S3_REGION").ok(),
                                        env("S3_ACCESS_KEY"),
                                        env("S3_SECRET_KEY"),
                                        &api_protocol),
                proxy: None,
            }
        },
        (cargo_registry::Env::Production, true) => {
            // Read-only mirrors don't need access key or secret key,
            // but they might have them. Definitely need bucket though.
            cargo_registry::Uploader::S3 {
                bucket: s3::Bucket::new(env("S3_BUCKET"),
                                        env::var("S3_REGION").ok(),
                                        env::var("S3_ACCESS_KEY").unwrap_or(String::new()),
                                        env::var("S3_SECRET_KEY").unwrap_or(String::new()),
                                        &api_protocol),
                proxy: None,
            }
        },
        (cargo_registry::Env::Development, _) => {
            if env::var("S3_BUCKET").is_ok() {
                println!("Using S3 uploader");
                cargo_registry::Uploader::S3 {
                    bucket: s3::Bucket::new(env("S3_BUCKET"),
                                            env::var("S3_REGION").ok(),
                                            env::var("S3_ACCESS_KEY").unwrap_or(String::new()),
                                            env::var("S3_SECRET_KEY").unwrap_or(String::new()),
                                            &api_protocol),
                    proxy: None,
                }
            } else {
                println!("Using local uploader, crate files will be in the dist directory");
                cargo_registry::Uploader::Local
            }
        },
        // See immediately before this match where we choose either prod or dev
        (cargo_registry::Env::Test, _) => unreachable!(),
    };

    let config = cargo_registry::Config {
        uploader: uploader,
        session_key: env("SESSION_KEY"),
        git_repo_checkout: checkout,
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_env,
        max_upload_size: 10 * 1024 * 1024,
        mirror: mirror,
        api_protocol: api_protocol,
    };
    let app = cargo_registry::App::new(&config);
    let app = cargo_registry::middleware(Arc::new(app));

    cargo_registry::categories::sync().unwrap();

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
