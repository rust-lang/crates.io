use crate::job_registry::JobRegistry;
use crate::storage;
use crate::util::{try_to_extract_panic_info, with_sentry_transaction};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use futures_util::FutureExt;
use sentry_core::{Hub, SentryFutureExt};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{Instrument, debug, error, info_span, warn};

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
        conn.transaction(|conn| {
            async move {
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
                        // TODO: Replace with flatten() once that stabilizes
                        .and_then(std::convert::identity)
                });

                let result = future
                    .instrument(span.clone())
                    .bind_hub(Hub::current())
                    .await;

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
            }
            .scope_boxed()
        })
        .await
    }
}
