// Runs enqueued background jobs
//
// This binary will loop until interrupted. Every second, it will attempt to
// run any jobs in the background queue. Panics if attempting to count
// available jobs fails.
//
// Usage:
//      cargo run --bin background-worker

#![deny(warnings, clippy::all, rust_2018_idioms)]

use cargo_registry::git::Repository;
use cargo_registry::{background_jobs::*, db};
use diesel::r2d2;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("Booting runner");

    let config = cargo_registry::Config::default();

    // We're only using 1 thread, so we only need 2 connections
    let db_config = r2d2::Pool::builder().max_size(2);
    let db_pool = db::diesel_pool(&config.db_url, config.env, db_config);

    let username = dotenv::var("GIT_HTTP_USER");
    let password = dotenv::var("GIT_HTTP_PWD");
    let credentials = match (username, password) {
        (Ok(u), Ok(p)) => Some((u, p)),
        _ => None,
    };

    println!("Cloning index");

    let repository = Repository::open(&config.index_location).expect("Failed to clone index");

    let environment = Environment::new(
        repository,
        credentials,
        db_pool.clone(),
        config.uploader,
        reqwest::Client::new(),
    );

    let runner = swirl::Runner::builder(db_pool, environment)
        .thread_count(1)
        .job_start_timeout(Duration::from_secs(10))
        .build();

    println!("Runner booted, running jobs");

    loop {
        runner
            .run_all_pending_jobs()
            .expect("Could not begin running jobs");
        sleep(Duration::from_secs(1));
    }
}
