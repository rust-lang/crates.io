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

#[macro_use]
extern crate tracing;

use anyhow::Context;
use crates_io::cloudfront::CloudFront;
use crates_io::db::DieselPool;
use crates_io::fastly::Fastly;
use crates_io::storage::Storage;
use crates_io::team_repo::TeamRepoImpl;
use crates_io::worker::{Environment, RunnerExt};
use crates_io::{config, Emails};
use crates_io::{db, ssh};
use crates_io_env_vars::var;
use crates_io_index::RepositoryConfig;
use crates_io_worker::Runner;
use deadpool_diesel::postgres::{Manager as DeadpoolManager, Pool as DeadpoolPool};
use deadpool_diesel::Runtime;
use diesel::r2d2;
use diesel::r2d2::ConnectionManager;
use reqwest::Client;
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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to initialize tokio runtime")?;

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

    if var("HEROKU")?.is_some() {
        ssh::write_known_hosts_file().unwrap();
    }

    let repository_config = RepositoryConfig::from_environment()?;

    let cloudfront = CloudFront::from_environment();
    let storage = Arc::new(Storage::from_config(&config.storage));

    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .expect("Couldn't build client");

    let emails = Emails::from_environment(&config);
    let fastly = Fastly::from_environment(client.clone());
    let team_repo = TeamRepoImpl::default();

    let connection_pool = r2d2::Pool::builder()
        .max_size(10)
        .min_idle(Some(0))
        .build_unchecked(ConnectionManager::new(&db_url));

    let manager = DeadpoolManager::new(db_url, Runtime::Tokio1);
    let deadpool = DeadpoolPool::builder(manager).max_size(10).build().unwrap();

    let environment = Environment::builder()
        .config(Arc::new(config))
        .repository_config(repository_config)
        .cloudfront(cloudfront)
        .fastly(fastly)
        .storage(storage)
        .connection_pool(DieselPool::new_background_worker(connection_pool.clone()))
        .deadpool(deadpool)
        .emails(emails)
        .team_repo(Box::new(team_repo))
        .build()?;

    let environment = Arc::new(environment);

    std::thread::spawn({
        let environment = environment.clone();
        move || {
            if let Err(err) = environment.lock_index() {
                warn!(%err, "Failed to clone index");
            };
        }
    });

    let runner = Runner::new(runtime.handle(), connection_pool, environment.clone())
        .configure_default_queue(|queue| queue.num_workers(5))
        .configure_queue("downloads", |queue| queue.num_workers(1))
        .configure_queue("repository", |queue| queue.num_workers(1))
        .register_crates_io_job_types()
        .start();

    info!("Runner booted, running jobs");
    runtime.block_on(runner.wait_for_shutdown());

    Ok(())
}
