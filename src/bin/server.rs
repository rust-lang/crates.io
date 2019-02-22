#![deny(warnings)]

use cargo_registry::{boot, build_handler, env, git, App, Config, Env};
use jemalloc_ctl;
use std::{
    env,
    fs::{self, File},
    sync::Arc,
};

use conduit_hyper::Service;

fn main() {
    let _ = jemalloc_ctl::set_background_thread(true);

    // Initialize logging
    env_logger::init();
    let config = Config::default();

    // If there isn't a git checkout containing the crate index repo at the path specified
    // by `GIT_REPO_CHECKOUT`, delete that directory and clone the repo specified by `GIT_REPO_URL`
    // into that directory instead. Uses the credentials specified in `GIT_HTTP_USER` and
    // `GIT_HTTP_PWD` via the `cargo_registry::git::credentials` function.
    let url = env("GIT_REPO_URL");
    let repo = match git2::Repository::open(&config.git_repo_checkout) {
        Ok(r) => r,
        Err(..) => {
            let _ = fs::remove_dir_all(&config.git_repo_checkout);
            fs::create_dir_all(&config.git_repo_checkout).unwrap();
            let mut cb = git2::RemoteCallbacks::new();
            cb.credentials(git::credentials);
            let mut opts = git2::FetchOptions::new();
            opts.remote_callbacks(cb);
            git2::build::RepoBuilder::new()
                .fetch_options(opts)
                .clone(&url, &config.git_repo_checkout)
                .unwrap()
        }
    };

    // All commits to the index registry made through crates.io will be made by bors, the Rust
    // community's friendly GitHub bot.
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "bors").unwrap();
    cfg.set_str("user.email", "bors@rust-lang.org").unwrap();

    let app = App::new(&config);
    let app = build_handler(Arc::new(app));

    // On every server restart, ensure the categories available in the database match
    // the information in *src/categories.toml*.
    let categories_toml = include_str!("../boot/categories.toml");
    boot::categories::sync(categories_toml).unwrap();

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
    let addr = ([127, 0, 0, 1], port).into();
    let server = Service::new(app, threads);

    println!("listening on port {}", port);

    // Creating this file tells heroku to tell nginx that the application is ready
    // to receive traffic.
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    // TODO: handle a graceful shutdown by just waiting for a SIG{INT,TERM}
    server.run(addr);
}
