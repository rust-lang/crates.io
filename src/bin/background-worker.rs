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
use crates_io::{db, env_optional, ssh};
use crates_io_index::{Repository, RepositoryConfig};
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    let _span = info_span!("swirl.run");

    info!("Booting runner");

    let config = config::Server::default();

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

    let job_start_timeout = env_optional("BACKGROUND_JOB_TIMEOUT").unwrap_or(30);

    info!("Cloning index");

    if dotenvy::var("HEROKU").is_ok() {
        ssh::write_known_hosts_file().unwrap();
    }

    let clone_start = Instant::now();
    let repository_config = RepositoryConfig::from_environment();
    let repository = Repository::open(&repository_config).expect("Failed to clone index");

    let clone_duration = clone_start.elapsed();
    info!(duration = ?clone_duration, "Index cloned");

    let cloudfront = CloudFront::from_environment();
    let fastly = Fastly::from_environment();
    let storage = Arc::new(Storage::from_config(&config.storage));

    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .expect("Couldn't build client");

    let environment = Environment::new(repository, client, cloudfront, fastly, storage);

    let environment = Arc::new(environment);

    let build_runner = || {
        let connection_pool = r2d2::Pool::builder()
            .max_size(10)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(&db_url));

        let connection_pool = DieselPool::new_background_worker(connection_pool);

        Runner::new(connection_pool, environment.clone())
            .num_workers(5)
            .job_start_timeout(Duration::from_secs(job_start_timeout))
            .register_crates_io_job_types()
    };

    let mut runner = build_runner();

    info!("Runner booted, running jobs");

    let mut failure_count = 0;

    loop {
        if let Err(e) = runner.run_all_pending_jobs() {
            failure_count += 1;
            if failure_count < 5 {
                warn!(?failure_count, err = ?e, "Error running jobs -- retrying");
                runner = build_runner();
            } else {
                panic!("Failed to begin running jobs 5 times. Restarting the process");
            }
        }
        sleep(Duration::from_secs(1));
    }
}
