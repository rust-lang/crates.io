#![deny(warnings)]

extern crate cargo_registry;
extern crate conduit_middleware;
extern crate civet;
extern crate git2;
extern crate env_logger;
extern crate s3;

use cargo_registry::{env, Env, Uploader, Replica};
use civet::Server;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;

#[allow(dead_code)]
fn main() {
    // Initialize logging
    env_logger::init().unwrap();

    // If there isn't a git checkout containing the crate index repo at the path specified
    // by `GIT_REPO_CHECKOUT`, delete that directory and clone the repo specified by `GIT_REPO_URL`
    // into that directory instead. Uses the credentials specified in `GIT_HTTP_USER` and
    // `GIT_HTTP_PWD` via the `cargo_registry::git::credentials` function.
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
                .clone(&url, &checkout)
                .unwrap()
        }
    };

    // All commits to the index registry made through crates.io will be made by bors, the Rust
    // community's friendly GitHub bot.
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "bors").unwrap();
    cfg.set_str("user.email", "bors@rust-lang.org").unwrap();

    let api_protocol = String::from("https");

    let mirror = if env::var("MIRROR").is_ok() {
        Replica::ReadOnlyMirror
    } else {
        Replica::Primary
    };

    let heroku = env::var("HEROKU").is_ok();
    let cargo_env = if heroku {
        Env::Production
    } else {
        Env::Development
    };

    let uploader = match (cargo_env, mirror) {
        (Env::Production, Replica::Primary) => {
            // `env` panics if these vars are not set, and in production for a primary instance,
            // that's what we want since we don't want to be able to start the server if the server
            // doesn't know where to upload crates.
            Uploader::S3 {
                bucket: s3::Bucket::new(
                    env("S3_BUCKET"),
                    env::var("S3_REGION").ok(),
                    env("S3_ACCESS_KEY"),
                    env("S3_SECRET_KEY"),
                    &api_protocol,
                ),
                proxy: None,
            }
        }
        (Env::Production, Replica::ReadOnlyMirror) => {
            // Read-only mirrors don't need access key or secret key since by definition,
            // they'll only need to read from a bucket, not upload.
            //
            // Read-only mirrors might have access key or secret key, so use them if those
            // environment variables are set.
            //
            // Read-only mirrors definitely need bucket though, so that they know where
            // to serve crate files from.
            Uploader::S3 {
                bucket: s3::Bucket::new(
                    env("S3_BUCKET"),
                    env::var("S3_REGION").ok(),
                    env::var("S3_ACCESS_KEY").unwrap_or(String::new()),
                    env::var("S3_SECRET_KEY").unwrap_or(String::new()),
                    &api_protocol,
                ),
                proxy: None,
            }
        }
        // In Development mode, either running as a primary instance or a read-only mirror
        _ => {
            if env::var("S3_BUCKET").is_ok() {
                // If we've set the `S3_BUCKET` variable to any value, use all of the values
                // for the related S3 environment variables and configure the app to upload to
                // and read from S3 like production does. All values except for bucket are
                // optional, like production read-only mirrors.
                println!("Using S3 uploader");
                Uploader::S3 {
                    bucket: s3::Bucket::new(
                        env("S3_BUCKET"),
                        env::var("S3_REGION").ok(),
                        env::var("S3_ACCESS_KEY").unwrap_or(String::new()),
                        env::var("S3_SECRET_KEY").unwrap_or(String::new()),
                        &api_protocol,
                    ),
                    proxy: None,
                }
            } else {
                // If we don't set the `S3_BUCKET` variable, we'll use a development-only
                // uploader that makes it possible to run and publish to a locally-running
                // crates.io instance without needing to set up an account and a bucket in S3.
                println!("Using local uploader, crate files will be in the dist directory");
                Uploader::Local
            }
        }
    };

    let config = cargo_registry::Config {
        uploader: uploader,
        session_key: env("SESSION_KEY"),
        git_repo_checkout: checkout,
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_env,
        max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
        mirror: mirror,
        api_protocol: api_protocol,
    };
    let app = cargo_registry::App::new(&config);
    let app = cargo_registry::middleware(Arc::new(app));

    // On every server restart, ensure the categories available in the database match
    // the information in *src/categories.toml*.
    let categories_toml = include_str!("../categories.toml");
    cargo_registry::categories::sync(&categories_toml).unwrap();

    let port = if heroku {
        8888
    } else {
        env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8888)
    };
    let threads = if cargo_env == Env::Development { 1 } else { 50 };
    let mut cfg = civet::Config::new();
    cfg.port(port).threads(threads).keep_alive(true);
    let _a = Server::start(cfg, app);

    println!("listening on port {}", port);

    // Creating this file tells heroku to tell nginx that the application is ready
    // to receive traffic.
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    // TODO: handle a graceful shutdown by just waiting for a SIG{INT,TERM}
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}
