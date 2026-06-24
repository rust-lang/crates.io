//! Runs enqueued background jobs
//!
//! This binary will loop until interrupted. It will run all jobs in the
//! background queue, waiting for a Postgres `NOTIFY` (or polling once per
//! second as a fallback) whenever the queue is empty. If we
//! are unable to spawn workers to run jobs (either because we couldn't connect
//! to the DB, an error occurred while loading, or we just never heard back from
//! the worker thread), we will rebuild the runner and try again up to 5 times.
//! After the 5th occurrence, we will panic.
//!
//! Usage:
//!      cargo run --bin background-worker

#[macro_use]
extern crate tracing;

use anyhow::{Context, anyhow};
use crates_io::app::create_database_pool;
use crates_io::cloudfront::CloudFront;
use crates_io::ssh;
use crates_io::storage::Storage;
use crates_io::worker::{Environment, RunnerExt};
use crates_io::{Emails, config};
use crates_io_docs_rs::RealDocsRsClient;
use crates_io_env_vars::{required_var, var};
use crates_io_fastly::Fastly;
use crates_io_github::{GitHubClient, RealGitHubClient};
use crates_io_github_app::{GitHubApp, GitHubAppClient};
use crates_io_index::RepositoryConfig;
use crates_io_og_image::OgImageGenerator;
use crates_io_team_repo::TeamRepoImpl;
use crates_io_worker::Runner;
use object_store::prefix::PrefixStore;
use reqwest::Client;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use url::Url;

fn main() -> anyhow::Result<()> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    let _span = info_span!("swirl.run");

    info!("Booting runner");

    let mut config = config::Server::from_environment()?;

    // Override the pool size to 10 for the background worker
    config.db.primary.pool_size = 10;

    // We run some long-running queries in the background worker, so we need to
    // increase the statement timeout a bit…
    config.db.primary.statement_timeout = Duration::from_secs(4 * 60 * 60);

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

    if var("HEROKU")?.is_some() {
        ssh::write_known_hosts_file()?;
    }

    let repository_config = RepositoryConfig::from_environment()?;

    let user_agent = crates_io_version::user_agent();
    let http_client = Client::builder().user_agent(user_agent).build()?;

    let cloudfront = CloudFront::from_environment();
    let storage = Arc::new(Storage::from_config(&config.storage));

    let downloads_archive_store = PrefixStore::new(storage.as_inner(), "archive/version-downloads");
    let downloads_archive_store = Box::new(downloads_archive_store);

    let emails = Emails::from_environment(&config);

    let fastly_api_token = var("FASTLY_API_TOKEN")?.map(Into::into);
    let fastly = fastly_api_token.map(Fastly::new);

    let team_repo = TeamRepoImpl::default();

    let docs_rs = RealDocsRsClient::from_environment().map(|cl| Box::new(cl) as _);

    let github: Arc<dyn GitHubClient> = Arc::new(RealGitHubClient::new(http_client.clone()));
    let index_sync_github_app = build_index_sync_github_app(config.index_archive_url.as_ref())?;
    let sync_github_app = build_sync_github_app()?;

    let deadpool = create_database_pool(&config.db.primary);

    let environment = Environment::builder()
        .config(Arc::new(config))
        .repository_config(repository_config)
        .maybe_cloudfront(cloudfront)
        .maybe_fastly(fastly)
        .storage(storage)
        .downloads_archive_store(downloads_archive_store)
        .deadpool(deadpool.clone())
        .emails(emails)
        .maybe_docs_rs(docs_rs)
        .team_repo(Box::new(team_repo))
        .maybe_index_sync_github_app(index_sync_github_app)
        .maybe_sync_github_app(sync_github_app)
        .github(github)
        .og_image_generator(OgImageGenerator::from_environment()?.with_oxipng())
        .build();

    let environment = Arc::new(environment);

    std::thread::spawn({
        let environment = environment.clone();
        move || {
            if let Err(err) = environment.lock_index() {
                warn!("Failed to clone index: {err}");
            };
        }
    });

    let runner = Runner::new(deadpool, environment.clone())
        .configure_default_queue(|queue| queue.num_workers(5))
        .configure_queue("downloads", |queue| queue.num_workers(1))
        .configure_queue("repository", |queue| queue.num_workers(1))
        .configure_queue("cloudfront", |queue| queue.num_workers(1))
        .register_crates_io_job_types();

    runtime.block_on(async {
        let handle = runner.start();
        crates_io::metrics::datadog::spawn(
            &environment.config,
            environment.deadpool.clone(),
            http_client,
        );

        info!("Runner booted, running jobs");
        handle.wait_for_shutdown().await
    });

    Ok(())
}

/// Builds the GitHub App client used to authenticate the archive index
/// push. Returns `None` when `GIT_ARCHIVE_REPO_URL` is unset, in which
/// case no archive push happens and no credentials are needed.
///
/// When the archive URL *is* set, both `GH_INDEX_SYNC_APP_CLIENT_ID`
/// and `GH_INDEX_SYNC_APP_PRIVATE_KEY` must also be present, and the
/// archive URL must include an `<org>` path segment.
fn build_index_sync_github_app(
    archive_url: Option<&Url>,
) -> anyhow::Result<Option<Arc<dyn GitHubApp>>> {
    let Some(archive_url) = archive_url else {
        return Ok(None);
    };

    let org = archive_url
        .path_segments()
        .and_then(|mut segments| segments.next())
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| anyhow!("GIT_ARCHIVE_REPO_URL is missing the org path segment"))?;

    let client_id = required_var("GH_INDEX_SYNC_APP_CLIENT_ID")?;
    let pem = required_var("GH_INDEX_SYNC_APP_PRIVATE_KEY")?;

    let client = GitHubAppClient::new(&client_id, &pem, org)?;
    Ok(Some(Arc::new(client)))
}

/// Builds the GitHub App client used to authenticate requests to
/// the users API. Returns `None` when `GH_SYNC_APP_CLIENT_ID` is unset. When it
/// is set, `GH_SYNC_APP_ORG` and `GH_SYNC_APP_PRIVATE_KEY` must also be present.
fn build_sync_github_app() -> anyhow::Result<Option<Arc<dyn GitHubApp>>> {
    let Some(client_id) = var("GH_SYNC_APP_CLIENT_ID")? else {
        return Ok(None);
    };

    let pem = required_var("GH_SYNC_APP_PRIVATE_KEY")?;
    let org = required_var("GH_SYNC_APP_ORG")?;

    let client = GitHubAppClient::new(&client_id, &pem, &org)?;
    Ok(Some(Arc::new(client)))
}
