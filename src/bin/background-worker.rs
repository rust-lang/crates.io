//! Runs enqueued background jobs
//!
//! This binary will loop until interrupted. It will run all jobs in the
//! background queue, sleeping for 1 second whenever the queue is empty. If we
//! are unable to spawn workers to run jobs (either because we couldn't connect
//! to the DB, an error occurred while loading, or we just never heard back from
//! the worker thread), we will rebuild the runner and try again up to 5 times.
//! After the 5th occurrence, we will panic.
//!
//! Usage:
//!      cargo run --bin background-worker

#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

use crates_io::cloudfront::CloudFront;
use crates_io::config;
use crates_io::db::DieselPool;
use crates_io::fastly::Fastly;
use crates_io::storage::Storage;
use crates_io::worker::swirl::Runner;
use crates_io::worker::{Environment, RunnerExt};
use crates_io::{db, ssh};
use crates_io_env_vars::{var, var_parsed};
use crates_io_index::RepositoryConfig;
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    let _span = info_span!("swirl.run");

    info!("Booting runner");

    let config = config::Server::from_environment()?;

    if config.db.are_all_read_only() {
        loop {
            warn!(
                "Cannot run background jobs with a read-only pool. Please scale background_worker \
                to 0 processes until the leader database is available."
            );
            sleep(Duration::from_secs(60));
        }
    }

    let db_url = db::connection_url(&config.db, config.db.primary.url.expose_secret());

    let job_start_timeout = var_parsed("BACKGROUND_JOB_TIMEOUT")?.unwrap_or(30);

    if var("HEROKU")?.is_some() {
        ssh::write_known_hosts_file().unwrap();
    }

    let repository_config = RepositoryConfig::from_environment()?;

    let cloudfront = CloudFront::from_environment();
    let fastly = Fastly::from_environment();
    let storage = Arc::new(Storage::from_config(&config.storage));

    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .expect("Couldn't build client");

    let connection_pool = r2d2::Pool::builder()
        .max_size(10)
        .min_idle(Some(0))
        .build_unchecked(ConnectionManager::new(&db_url));

    let connection_pool = DieselPool::new_background_worker(connection_pool);

    let environment = Environment::new(
        repository_config,
        client,
        cloudfront,
        fastly,
        storage,
        connection_pool.clone(),
    );

    let environment = Arc::new(environment);

    std::thread::spawn({
        let environment = environment.clone();
        move || {
            if let Err(err) = environment.lock_index() {
                warn!(%err, "Failed to clone index");
            };
        }
    });

    let runner = Runner::new(connection_pool, environment.clone())
        .num_workers(5)
        .job_start_timeout(Duration::from_secs(job_start_timeout))
        .register_crates_io_job_types();

    info!("Runner booted, running jobs");

    loop {
        if let Err(err) = runner.run_all_pending_jobs() {
            warn!(%err, "Failed to run background jobs");
        }
        sleep(Duration::from_secs(1));
    }
}
