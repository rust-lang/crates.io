use crate::worker::swirl::{storage, BackgroundJob};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};
use futures_util::future::join_all;
use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe, PanicInfo};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

type RunTaskFn<Context> = dyn Fn(Context, serde_json::Value) -> anyhow::Result<()> + Send + Sync;

type JobRegistry<Context> = HashMap<String, Arc<RunTaskFn<Context>>>;

fn runnable<J: BackgroundJob>(ctx: J::Context, payload: serde_json::Value) -> anyhow::Result<()> {
    let job: J = serde_json::from_value(payload)?;
    Handle::current().block_on(job.run(ctx))
}

/// The core runner responsible for locking and running jobs
pub struct Runner<Context> {
    rt_handle: Handle,
    connection_pool: ConnectionPool,
    num_workers: usize,
    job_registry: JobRegistry<Context>,
    context: Context,
    poll_interval: Duration,
    shutdown_when_queue_empty: bool,
}

impl<Context: Clone + Send + 'static> Runner<Context> {
    pub fn new(rt_handle: &Handle, connection_pool: ConnectionPool, context: Context) -> Self {
        Self {
            rt_handle: rt_handle.clone(),
            connection_pool,
            num_workers: 1,
            job_registry: Default::default(),
            context,
            poll_interval: DEFAULT_POLL_INTERVAL,
            shutdown_when_queue_empty: false,
        }
    }

    /// Set the number of workers to spawn.
    pub fn num_workers(mut self, num_workers: usize) -> Self {
        self.num_workers = num_workers;
        self
    }

    /// Set the interval after which each worker polls for new jobs.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Register a new job type for this job runner.
    pub fn register_job_type<J: BackgroundJob<Context = Context>>(mut self) -> Self {
        self.job_registry
            .insert(J::JOB_NAME.to_string(), Arc::new(runnable::<J>));

        self
    }

    /// Set the runner to shut down when the background job queue is empty.
    pub fn shutdown_when_queue_empty(mut self) -> Self {
        self.shutdown_when_queue_empty = true;
        self
    }

    /// Start the background workers.
    ///
    /// This returns a `RunningRunner` which can be used to wait for the workers to shutdown.
    pub fn start(&self) -> RunHandle {
        let handles = (0..self.num_workers)
            .map(|i| {
                let name = format!("background-worker-{i}");
                info!(worker.name = %name, "Starting worker…");

                let worker = Worker {
                    connection_pool: self.connection_pool.clone(),
                    context: self.context.clone(),
                    job_registry: self.job_registry.clone(),
                    shutdown_when_queue_empty: self.shutdown_when_queue_empty,
                    poll_interval: self.poll_interval,
                };

                self.rt_handle.spawn_blocking(move || {
                    info_span!("worker", worker.name = %name).in_scope(|| worker.run())
                })
            })
            .collect();

        RunHandle { handles }
    }

    fn connection(&self) -> Result<PooledConn, PoolError> {
        self.connection_pool.get()
    }

    /// Check if any jobs in the queue have failed.
    ///
    /// This function is intended for use in tests and will return an error if
    /// any jobs have failed.
    pub fn check_for_failed_jobs(&self) -> anyhow::Result<()> {
        let failed_jobs = storage::failed_job_count(&mut *self.connection()?)?;
        if failed_jobs == 0 {
            Ok(())
        } else {
            Err(anyhow!("{failed_jobs} jobs failed"))
        }
    }
}

pub struct RunHandle {
    handles: Vec<JoinHandle<()>>,
}

impl RunHandle {
    /// Wait for all background workers to shut down.
    pub async fn wait_for_shutdown(self) {
        join_all(self.handles).await.into_iter().for_each(|result| {
            if let Err(error) = result {
                warn!(%error, "Background worker task panicked");
            }
        });
    }
}

struct Worker<Context> {
    connection_pool: ConnectionPool,
    context: Context,
    job_registry: JobRegistry<Context>,
    shutdown_when_queue_empty: bool,
    poll_interval: Duration,
}

impl<Context: Clone + Send + 'static> Worker<Context> {
    /// Run background jobs forever, or until the queue is empty if `shutdown_when_queue_empty` is set.
    pub fn run(&self) {
        loop {
            match self.run_next_job() {
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
                    std::thread::sleep(self.poll_interval);
                }
                Err(error) => {
                    error!(%error, "Failed to run job");
                    std::thread::sleep(self.poll_interval);
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
    fn run_next_job(&self) -> anyhow::Result<Option<i64>> {
        let conn = &mut *self.connection_pool.get()?;

        conn.transaction(|conn| {
            debug!("Looking for next background worker job…");
            let Some(job) = storage::find_next_unlocked_job(conn).optional()? else {
                return Ok(None);
            };

            let span = info_span!("job", job.id = %job.id, job.typ = %job.job_type);
            let _enter = span.enter();

            let job_id = job.id;
            debug!("Running job…");

            let context = self.context.clone();

            let result = with_sentry_transaction(&job.job_type, || {
                catch_unwind(AssertUnwindSafe(|| {
                    let run_task_fn = self
                        .job_registry
                        .get(&job.job_type)
                        .ok_or_else(|| anyhow!("Unknown job type {}", job.job_type))?;

                    run_task_fn(context, job.data)
                }))
                .map_err(|e| try_to_extract_panic_info(&e))
                // TODO: Replace with flatten() once that stabilizes
                .and_then(std::convert::identity)
            });

            match result {
                Ok(_) => {
                    debug!("Deleting successful job…");
                    storage::delete_successful_job(conn, job_id)?
                }
                Err(error) => {
                    warn!(%error, "Failed to run job");
                    storage::update_failed_job(conn, job_id);
                }
            }

            Ok(Some(job_id))
        })
    }
}

fn with_sentry_transaction<F, R, E>(transaction_name: &str, callback: F) -> Result<R, E>
where
    F: FnOnce() -> Result<R, E>,
{
    let tx_ctx = sentry::TransactionContext::new(transaction_name, "swirl.perform");
    let tx = sentry::start_transaction(tx_ctx);

    let result = sentry::with_scope(|scope| scope.set_span(Some(tx.clone().into())), callback);

    tx.set_status(match result.is_ok() {
        true => sentry::protocol::SpanStatus::Ok,
        false => sentry::protocol::SpanStatus::UnknownError,
    });
    tx.finish();

    result
}

/// Try to figure out what's in the box, and print it if we can.
///
/// The actual error type we will get from `panic::catch_unwind` is really poorly documented.
/// However, the `panic::set_hook` functions deal with a `PanicInfo` type, and its payload is
/// documented as "commonly but not always `&'static str` or `String`". So we can try all of those,
/// and give up if we didn't get one of those three types.
fn try_to_extract_panic_info(info: &(dyn Any + Send + 'static)) -> anyhow::Error {
    if let Some(x) = info.downcast_ref::<PanicInfo<'_>>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<&'static str>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<String>() {
        anyhow!("job panicked: {x}")
    } else {
        anyhow!("job panicked")
    }
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;

    use super::*;
    use crate::schema::background_jobs;
    use async_trait::async_trait;
    use crates_io_test_db::TestDatabase;
    use diesel::r2d2;
    use diesel::r2d2::ConnectionManager;
    use std::sync::Arc;
    use tokio::runtime::Runtime;
    use tokio::sync::Barrier;

    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn job_exists(id: i64, conn: &mut PgConnection) -> bool {
        background_jobs::table
            .find(id)
            .select(background_jobs::id)
            .get_result::<i64>(conn)
            .optional()
            .unwrap()
            .is_some()
    }

    fn job_is_locked(id: i64, conn: &mut PgConnection) -> bool {
        background_jobs::table
            .find(id)
            .select(background_jobs::id)
            .for_update()
            .skip_locked()
            .get_result::<i64>(conn)
            .optional()
            .unwrap()
            .is_none()
    }

    #[test]
    fn jobs_are_locked_when_fetched() {
        #[derive(Clone)]
        struct TestContext {
            job_started_barrier: Arc<Barrier>,
            assertions_finished_barrier: Arc<Barrier>,
        }

        #[derive(Serialize, Deserialize)]
        struct TestJob;

        #[async_trait]
        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = TestContext;

            async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
                ctx.job_started_barrier.wait().await;
                ctx.assertions_finished_barrier.wait().await;
                Ok(())
            }
        }

        let test_database = TestDatabase::new();

        let test_context = TestContext {
            job_started_barrier: Arc::new(Barrier::new(2)),
            assertions_finished_barrier: Arc::new(Barrier::new(2)),
        };

        let rt = runtime();
        let runner =
            runner(&rt, test_database.url(), test_context.clone()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        let job_id = TestJob.enqueue(&mut conn).unwrap();

        assert!(job_exists(job_id, &mut conn));
        assert!(!job_is_locked(job_id, &mut conn));

        let runner = runner.start();
        rt.block_on(test_context.job_started_barrier.wait());

        assert!(job_exists(job_id, &mut conn));
        assert!(job_is_locked(job_id, &mut conn));

        rt.block_on(test_context.assertions_finished_barrier.wait());
        rt.block_on(runner.wait_for_shutdown());

        assert!(!job_exists(job_id, &mut conn));
    }

    #[test]
    fn jobs_are_deleted_when_successfully_run() {
        #[derive(Serialize, Deserialize)]
        struct TestJob;

        #[async_trait]
        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = ();

            async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
                Ok(())
            }
        }

        fn remaining_jobs(conn: &mut PgConnection) -> i64 {
            background_jobs::table
                .count()
                .get_result(&mut *conn)
                .unwrap()
        }

        let test_database = TestDatabase::new();

        let rt = runtime();
        let runner = runner(&rt, test_database.url(), ()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        assert_eq!(remaining_jobs(&mut conn), 0);

        TestJob.enqueue(&mut conn).unwrap();
        assert_eq!(remaining_jobs(&mut conn), 1);

        let runner = runner.start();
        rt.block_on(runner.wait_for_shutdown());
        assert_eq!(remaining_jobs(&mut conn), 0);
    }

    #[test]
    fn failed_jobs_do_not_release_lock_before_updating_retry_time() {
        #[derive(Clone)]
        struct TestContext {
            job_started_barrier: Arc<Barrier>,
        }

        #[derive(Serialize, Deserialize)]
        struct TestJob;

        #[async_trait]
        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = TestContext;

            async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
                ctx.job_started_barrier.wait().await;
                panic!();
            }
        }

        let test_database = TestDatabase::new();

        let test_context = TestContext {
            job_started_barrier: Arc::new(Barrier::new(2)),
        };

        let rt = runtime();
        let runner =
            runner(&rt, test_database.url(), test_context.clone()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        TestJob.enqueue(&mut conn).unwrap();

        let runner = runner.start();
        rt.block_on(test_context.job_started_barrier.wait());

        // `SKIP LOCKED` is intentionally omitted here, so we block until
        // the lock on the first job is released.
        // If there is any point where the row is unlocked, but the retry
        // count is not updated, we will get a row here.
        let available_jobs = background_jobs::table
            .select(background_jobs::id)
            .filter(background_jobs::retries.eq(0))
            .for_update()
            .load::<i64>(&mut *conn)
            .unwrap();
        assert_eq!(available_jobs.len(), 0);

        // Sanity check to make sure the job actually is there
        let total_jobs_including_failed = background_jobs::table
            .select(background_jobs::id)
            .for_update()
            .load::<i64>(&mut *conn)
            .unwrap();
        assert_eq!(total_jobs_including_failed.len(), 1);

        rt.block_on(runner.wait_for_shutdown());
    }

    #[test]
    fn panicking_in_jobs_updates_retry_counter() {
        #[derive(Serialize, Deserialize)]
        struct TestJob;

        #[async_trait]
        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = ();

            async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
                panic!()
            }
        }

        let test_database = TestDatabase::new();

        let rt = runtime();
        let runner = runner(&rt, test_database.url(), ()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();

        let job_id = TestJob.enqueue(&mut conn).unwrap();

        let runner = runner.start();
        rt.block_on(runner.wait_for_shutdown());

        let tries = background_jobs::table
            .find(job_id)
            .select(background_jobs::retries)
            .for_update()
            .first::<i32>(&mut *conn)
            .unwrap();
        assert_eq!(tries, 1);
    }

    fn runner<Context: Clone + Send + 'static>(
        runtime: &Runtime,
        database_url: &str,
        context: Context,
    ) -> Runner<Context> {
        let connection_pool = Pool::builder()
            .max_size(4)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(database_url));

        Runner::new(runtime.handle(), connection_pool, context)
            .num_workers(2)
            .shutdown_when_queue_empty()
    }
}
