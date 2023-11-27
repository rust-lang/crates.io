use crate::job_registry::{runnable, JobRegistry};
use crate::worker::Worker;
use crate::{storage, BackgroundJob};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};
use futures_util::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::{info, info_span, warn};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
pub type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

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

#[cfg(test)]
mod tests {
    use diesel::prelude::*;

    use super::*;
    use crate::schema::background_jobs;
    use async_trait::async_trait;
    use crates_io_test_db::TestDatabase;
    use diesel::r2d2::ConnectionManager;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::Barrier;

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

    #[tokio::test]
    async fn jobs_are_locked_when_fetched() {
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

        let runner =
            runner(test_database.url(), test_context.clone()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        let job_id = TestJob.enqueue(&mut conn).unwrap();

        assert!(job_exists(job_id, &mut conn));
        assert!(!job_is_locked(job_id, &mut conn));

        let runner = runner.start();
        test_context.job_started_barrier.wait().await;

        assert!(job_exists(job_id, &mut conn));
        assert!(job_is_locked(job_id, &mut conn));

        test_context.assertions_finished_barrier.wait().await;
        runner.wait_for_shutdown().await;

        assert!(!job_exists(job_id, &mut conn));
    }

    #[tokio::test]
    async fn jobs_are_deleted_when_successfully_run() {
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

        let runner = runner(test_database.url(), ()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        assert_eq!(remaining_jobs(&mut conn), 0);

        TestJob.enqueue(&mut conn).unwrap();
        assert_eq!(remaining_jobs(&mut conn), 1);

        let runner = runner.start();
        runner.wait_for_shutdown().await;
        assert_eq!(remaining_jobs(&mut conn), 0);
    }

    #[tokio::test]
    async fn failed_jobs_do_not_release_lock_before_updating_retry_time() {
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

        let runner =
            runner(test_database.url(), test_context.clone()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();
        TestJob.enqueue(&mut conn).unwrap();

        let runner = runner.start();
        test_context.job_started_barrier.wait().await;

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

        runner.wait_for_shutdown().await;
    }

    #[tokio::test]
    async fn panicking_in_jobs_updates_retry_counter() {
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

        let runner = runner(test_database.url(), ()).register_job_type::<TestJob>();

        let mut conn = runner.connection().unwrap();

        let job_id = TestJob.enqueue(&mut conn).unwrap();

        let runner = runner.start();
        runner.wait_for_shutdown().await;

        let tries = background_jobs::table
            .find(job_id)
            .select(background_jobs::retries)
            .for_update()
            .first::<i32>(&mut *conn)
            .unwrap();
        assert_eq!(tries, 1);
    }

    fn runner<Context: Clone + Send + 'static>(
        database_url: &str,
        context: Context,
    ) -> Runner<Context> {
        let connection_pool = Pool::builder()
            .max_size(4)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(database_url));

        Runner::new(&Handle::current(), connection_pool, context)
            .num_workers(2)
            .shutdown_when_queue_empty()
    }
}
