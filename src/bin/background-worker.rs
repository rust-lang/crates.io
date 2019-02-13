// Runs enqueued background jobs
//
// This binary will loop until interrupted. Every second, it will attempt to
// run any jobs in the background queue. Panics if attempting to count
// available jobs fails.
//
// Usage:
//      cargo run --bin background-worker

#![deny(warnings)]

use cargo_registry::{background, background_jobs::*, db};
use diesel::r2d2;
use std::env;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let config = cargo_registry::Config::default();

    // We're only using 1 thread, so we only need 2 connections
    let db_config = r2d2::Pool::builder().max_size(1);
    let db_pool = db::diesel_pool(&config.db_url, config.env, db_config);

    let username = env::var("GIT_HTTP_USER");
    let password = env::var("GIT_HTTP_PWD");
    let credentials = match (username, password) {
        (Ok(u), Ok(p)) => Some((u, p)),
        _ => None,
    };
    let environment = Environment::new(config.index_location, credentials, db_pool.clone());

    let builder = background::Runner::builder(db_pool, environment).thread_count(1);
    let runner = job_runner(builder);

    println!("Runner booted, running jobs");

    loop {
        runner
            .run_all_pending_jobs()
            .expect("Could not begin running jobs");
        sleep(Duration::from_secs(1));
    }
}
