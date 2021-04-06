//! Runs enqueued background jobs
//!
//! This binary will loop until interrupted. It will run all jobs in the
//! background queue, sleeping for 1 second whenever the queue is empty. If we
//! are unable to spawn workers to run jobs (either because we couldn't connect
//! to the DB, an error occurred while loading, or we just never heard back from
//! the worker thread), we will rebuild the runner and try again up to 5 times.
//! After the 5th occurrance, we will panic.
//!
//! Usage:
//!      cargo run --bin background-worker

#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::email::Emails;
use cargo_registry::git::{Repository, RepositoryConfig};
use cargo_registry::{background_jobs::*, db};
use diesel::r2d2;
use reqwest::blocking::Client;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("Booting runner");

    let config = cargo_registry::Config::default();
    let db_url = db::connection_url(&config.db_url);

    let job_start_timeout = dotenv::var("BACKGROUND_JOB_TIMEOUT")
        .unwrap_or_else(|_| "30".into())
        .parse()
        .expect("Invalid value for `BACKGROUND_JOB_TIMEOUT`");

    println!("Cloning index");

    let repository_config = RepositoryConfig::from_environment();
    let repository = Arc::new(Mutex::new(
        Repository::open(&repository_config).expect("Failed to clone index"),
    ));
    println!("Index cloned");

    let emails = Emails::from_environment();
    let emails = Arc::new(emails);

    let build_runner = || {
        let environment = Environment::new_shared(
            repository.clone(),
            config.uploader.clone(),
            Client::new(),
            emails.clone(),
        );
        let db_config = r2d2::Pool::builder().min_idle(Some(0));
        swirl::Runner::builder(environment)
            .connection_pool_builder(&db_url, db_config)
            .job_start_timeout(Duration::from_secs(job_start_timeout))
            .build()
    };
    let mut runner = build_runner();

    println!("Runner booted, running jobs");

    let mut failure_count = 0;

    loop {
        if let Err(e) = runner.run_all_pending_jobs() {
            failure_count += 1;
            if failure_count < 5 {
                eprintln!(
                    "Error running jobs (n = {}) -- retrying: {:?}",
                    failure_count, e,
                );
                runner = build_runner();
            } else {
                panic!("Failed to begin running jobs 5 times. Restarting the process");
            }
        }
        sleep(Duration::from_secs(1));
    }
}
