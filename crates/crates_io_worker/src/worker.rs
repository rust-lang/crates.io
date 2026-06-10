use crate::job_registry::JobRegistry;
use crate::storage;
use crate::util::{try_to_extract_panic_info, with_sentry_transaction};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use futures_util::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{Instrument, debug, error, info_span, warn};

pub struct Worker<Context> {
    pub(crate) connection_pool: Pool<AsyncPgConnection>,
    pub(crate) context: Context,
    pub(crate) job_registry: Arc<JobRegistry<Context>>,
    pub(crate) shutdown_when_queue_empty: bool,
    pub(crate) poll_interval: Duration,
    /// Signaled by the [`listener`](crate::listener) when a new job is enqueued.
    pub(crate) notify: Arc<Notify>,
}

impl<Context: Clone + Send + Sync + 'static> Worker<Context> {
    /// Run background jobs forever, or until the queue is empty if `shutdown_when_queue_empty` is set.
    pub async fn run(&self) {
        loop {
            // Register interest in notifications *before* querying the database,
            // so that a notification arriving while we run or look up a job is
            // not missed.
            let mut notified = std::pin::pin!(self.notify.notified());
            notified.as_mut().enable();

            match self.run_next_job().await {
                // A job was run, so immediately look for the next one.
                Ok(Some(_)) => continue,
                Ok(None) if self.shutdown_when_queue_empty => {
                    debug!("No pending background worker jobs found. Shutting down the worker…");
                    break;
                }
                Ok(None) => {
                    debug!(
                        "No pending background worker jobs found. Waiting for a notification, or polling again in {:?}…",
                        self.poll_interval
                    );
                    // Wake up as soon as a new job is enqueued, but keep polling
                    // as a fallback so that retriable jobs are eventually picked
                    // up even without a notification.
                    tokio::select! {
                        _ = notified => {}
                        _ = sleep(self.poll_interval) => {}
                    }
                }
                Err(error) => {
                    error!("Failed to run job: {error}");
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
        let mut conn = self.connection_pool.get().await?;

        let job_types = job_registry.job_types();
        conn.transaction(async |conn| {
            debug!("Looking for next background worker job…");
            let Some(job) = storage::find_next_unlocked_job(conn, &job_types)
                .await
                .optional()?
            else {
                return Ok(None);
            };

            let span = info_span!("job", job.id = %job.id, job.typ = %job.job_type);

            let job_id = job.id;
            debug!("Running job…");

            let future = with_sentry_transaction(&job.job_type, async || {
                let run_task_fn = job_registry
                    .get(&job.job_type)
                    .ok_or_else(|| anyhow!("Unknown job type {}", job.job_type))?;

                AssertUnwindSafe(run_task_fn(context, job.data))
                    .catch_unwind()
                    .await
                    .map_err(|e| try_to_extract_panic_info(&e))
                    .flatten()
            });

            let result = future.instrument(span.clone()).await;

            let _enter = span.enter();
            match result {
                Ok(_) => {
                    debug!("Deleting successful job…");
                    storage::delete_successful_job(conn, job_id).await?
                }
                Err(error) => {
                    warn!("Failed to run job: {error}");
                    storage::update_failed_job(conn, job_id).await;
                }
            }

            Ok(Some(job_id))
        })
        .await
    }
}
