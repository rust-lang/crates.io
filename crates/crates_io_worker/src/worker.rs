use crate::job_registry::JobRegistry;
use crate::storage;
use crate::util::{try_to_extract_panic_info, with_sentry_transaction};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use futures_util::FutureExt;
use sentry_core::{Hub, SentryFutureExt};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::spawn_blocking;
use tokio::time::sleep;
use tracing::{debug, error, info_span, warn};

pub struct Worker<Context> {
    pub(crate) connection_pool: Pool<AsyncPgConnection>,
    pub(crate) context: Context,
    pub(crate) job_registry: Arc<JobRegistry<Context>>,
    pub(crate) shutdown_when_queue_empty: bool,
    pub(crate) poll_interval: Duration,
}

impl<Context: Clone + Send + Sync + 'static> Worker<Context> {
    /// Run background jobs forever, or until the queue is empty if `shutdown_when_queue_empty` is set.
    pub async fn run(&self) {
        loop {
            match self.run_next_job().await {
                Ok(Some(_)) => {}
                Ok(None) if self.shutdown_when_queue_empty => {
                    debug!("No pending background worker jobs found. Shutting down the worker…");
                    break;
                }
                Ok(None) => {
                    debug!(
                        "No pending background worker jobs found. Polling again in {:?}…",
                        self.poll_interval
                    );
                    sleep(self.poll_interval).await;
                }
                Err(error) => {
                    let error = format!("{error:#}");
                    error!(error, "Failed to run job");
                    sleep(self.poll_interval).await;
                }
            }
        }
    }

    /// Run the next job in the queue, if there is one.
    ///
    /// Returns:
    /// - `Ok(Some(job_id))` if a job was run
    /// - `Ok(None)` if no jobs were waiting
    /// - `Err(...)` if there was an error retrieving the job
    async fn run_next_job(&self) -> anyhow::Result<Option<i64>> {
        let context = self.context.clone();
        let job_registry = self.job_registry.clone();
        let conn = self.connection_pool.get().await?;

        spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

            let job_types = job_registry.job_types();
            conn.transaction(|conn| {
                debug!("Looking for next background worker job…");
                let Some(job) = storage::find_next_unlocked_job(conn, &job_types).optional()?
                else {
                    return Ok(None);
                };

                let span = info_span!("job", job.id = %job.id, job.typ = %job.job_type);
                let _enter = span.enter();

                let job_id = job.id;
                debug!("Running job…");

                let future = with_sentry_transaction(&job.job_type, || async {
                    let run_task_fn = job_registry
                        .get(&job.job_type)
                        .ok_or_else(|| anyhow!("Unknown job type {}", job.job_type))?;

                    AssertUnwindSafe(run_task_fn(context, job.data))
                        .catch_unwind()
                        .await
                        .map_err(|e| try_to_extract_panic_info(&e))
                        // TODO: Replace with flatten() once that stabilizes
                        .and_then(std::convert::identity)
                });

                let result = Handle::current().block_on(future.bind_hub(Hub::current()));

                match result {
                    Ok(_) => {
                        debug!("Deleting successful job…");
                        storage::delete_successful_job(conn, job_id)?
                    }
                    Err(error) => {
                        let error = format!("{error:#}");
                        warn!(error, "Failed to run job");
                        storage::update_failed_job(conn, job_id);
                    }
                }

                Ok(Some(job_id))
            })
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))?
    }
}
