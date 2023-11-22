use crate::db::{DieselPool, DieselPooledConn, PoolError};
use crate::worker::swirl::errors::FetchError;
use crate::worker::swirl::{storage, BackgroundJob};
use anyhow::anyhow;
use diesel::connection::{AnsiTransactionManager, TransactionManager};
use diesel::prelude::*;
use diesel::result::Error::RollbackTransaction;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe, PanicInfo};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::time::Duration;
use threadpool::ThreadPool;

const DEFAULT_JOB_START_TIMEOUT: Duration = Duration::from_secs(30);

type RunTaskFn<Context> = dyn Fn(&Context, serde_json::Value) -> anyhow::Result<()> + Send + Sync;

type JobRegistry<Context> = Arc<RwLock<HashMap<String, Box<RunTaskFn<Context>>>>>;

fn runnable<J: BackgroundJob>(env: &J::Context, payload: serde_json::Value) -> anyhow::Result<()> {
    let job: J = serde_json::from_value(payload)?;
    job.run(env)
}

/// The core runner responsible for locking and running jobs
pub struct Runner<Context> {
    connection_pool: DieselPool,
    thread_pool: ThreadPool,
    job_registry: JobRegistry<Context>,
    environment: Context,
    job_start_timeout: Duration,
}

impl<Context: Clone + Send + 'static> Runner<Context> {
    pub fn new(connection_pool: DieselPool, environment: Context) -> Self {
        Self {
            connection_pool,
            thread_pool: ThreadPool::new(1),
            job_registry: Default::default(),
            environment,
            job_start_timeout: DEFAULT_JOB_START_TIMEOUT,
        }
    }

    pub fn num_workers(mut self, num_workers: usize) -> Self {
        self.thread_pool.set_num_threads(num_workers);
        self
    }

    pub fn job_start_timeout(mut self, job_start_timeout: Duration) -> Self {
        self.job_start_timeout = job_start_timeout;
        self
    }

    pub fn register_job_type<J: BackgroundJob<Context = Context>>(self) -> Self {
        self.job_registry
            .write()
            .insert(J::JOB_NAME.to_string(), Box::new(runnable::<J>));

        self
    }

    /// Runs all pending jobs in the queue
    ///
    /// This function will return once all jobs in the queue have begun running,
    /// but does not wait for them to complete. When this function returns, at
    /// least one thread will have tried to acquire a new job, and found there
    /// were none in the queue.
    pub fn run_all_pending_jobs(&self) -> Result<(), FetchError> {
        use std::cmp::max;

        let max_threads = self.thread_pool.max_count();
        let (sender, receiver) = sync_channel(max_threads);
        let mut pending_messages = 0;
        loop {
            let available_threads = max_threads - self.thread_pool.active_count();

            let jobs_to_queue = if pending_messages == 0 {
                // If we have no queued jobs talking to us, and there are no
                // available threads, we still need to queue at least one job
                // or we'll never receive a message
                max(available_threads, 1)
            } else {
                available_threads
            };

            for _ in 0..jobs_to_queue {
                let worker = Worker {
                    connection_pool: self.connection_pool.clone(),
                    environment: self.environment.clone(),
                    job_registry: self.job_registry.clone(),
                    sender: sender.clone(),
                };

                self.thread_pool.execute(move || worker.run_next_job())
            }

            pending_messages += jobs_to_queue;
            match receiver.recv_timeout(self.job_start_timeout)? {
                Event::Working => pending_messages -= 1,
                Event::NoJobAvailable => return Ok(()),
                Event::ErrorLoadingJob(e) => return Err(FetchError::FailedLoadingJob(e)),
                Event::FailedToAcquireConnection(e) => {
                    return Err(FetchError::NoDatabaseConnection(e));
                }
            }
        }
    }

    fn connection(&self) -> Result<DieselPooledConn, PoolError> {
        self.connection_pool.get()
    }

    /// Waits for all running jobs to complete, and returns an error if any
    /// failed
    ///
    /// This function is intended for use in tests. If any jobs have failed, it
    /// will return `swirl::JobsFailed` with the number of jobs that failed.
    ///
    /// If any other unexpected errors occurred, such as panicked worker threads
    /// or an error loading the job count from the database, an opaque error
    /// will be returned.
    // FIXME: Only public for `src/tests/util/test_app.rs`
    pub fn check_for_failed_jobs(&self) -> anyhow::Result<()> {
        self.wait_for_jobs()?;
        let failed_jobs = storage::failed_job_count(&mut *self.connection()?)?;
        if failed_jobs == 0 {
            Ok(())
        } else {
            Err(anyhow!("{failed_jobs} jobs failed"))
        }
    }

    fn wait_for_jobs(&self) -> anyhow::Result<()> {
        self.thread_pool.join();
        let panic_count = self.thread_pool.panic_count();
        if panic_count == 0 {
            Ok(())
        } else {
            Err(anyhow!("{panic_count} threads panicked"))
        }
    }
}

struct Worker<Context> {
    connection_pool: DieselPool,
    environment: Context,
    job_registry: JobRegistry<Context>,
    sender: SyncSender<Event>,
}

impl<Context: Clone + Send + 'static> Worker<Context> {
    fn run_next_job(&self) {
        let conn = &mut *match self.connection_pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                // TODO: Review error handling and possibly drop all usage of `let _ =` in this file
                let _ = self.sender.send(Event::FailedToAcquireConnection(e));
                return;
            }
        };

        let job_run_result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
            debug!("Looking for next background worker job…");
            let job = match storage::find_next_unlocked_job(conn).optional() {
                Ok(Some(j)) => {
                    let _ = self.sender.send(Event::Working);
                    j
                }
                Ok(None) => {
                    let _ = self.sender.send(Event::NoJobAvailable);
                    return Ok(());
                }
                Err(e) => {
                    let _ = self.sender.send(Event::ErrorLoadingJob(e));
                    return Err(RollbackTransaction);
                }
            };

            let span = info_span!("job", job.id = %job.id, job.typ = %job.job_type);
            let _enter = span.enter();

            let job_id = job.id;
            debug!("Running job…");

            let initial_depth = get_transaction_depth(conn)?;
            if initial_depth != 1 {
                warn!(%initial_depth, "Unexpected initial transaction depth detected");
            }

            let result = with_sentry_transaction(&job.job_type, || {
                catch_unwind(AssertUnwindSafe(|| {
                    let job_registry = self.job_registry.read();
                    let run_task_fn = job_registry
                        .get(&job.job_type)
                        .ok_or_else(|| anyhow!("Unknown job type {}", job.job_type))?;

                    run_task_fn(&self.environment, job.data)
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
            Ok(())
        });

        match job_run_result {
            Ok(_) | Err(RollbackTransaction) => {}
            Err(e) => {
                panic!("Failed to update job: {e:?}");
            }
        }
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

#[derive(Debug)]
enum Event {
    Working,
    NoJobAvailable,
    ErrorLoadingJob(diesel::result::Error),
    FailedToAcquireConnection(PoolError),
}

fn get_transaction_depth(conn: &mut PgConnection) -> QueryResult<u32> {
    let transaction_manager = AnsiTransactionManager::transaction_manager_status_mut(conn);
    Ok(transaction_manager
        .transaction_depth()?
        .map(u32::from)
        .unwrap_or(0))
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
    use crates_io_test_db::TestDatabase;
    use diesel::r2d2;
    use diesel::r2d2::ConnectionManager;
    use std::sync::{Arc, Barrier};

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

        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = TestContext;

            fn run(&self, ctx: &Self::Context) -> anyhow::Result<()> {
                ctx.job_started_barrier.wait();
                ctx.assertions_finished_barrier.wait();
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

        runner.run_all_pending_jobs().unwrap();
        test_context.job_started_barrier.wait();

        assert!(job_exists(job_id, &mut conn));
        assert!(job_is_locked(job_id, &mut conn));

        test_context.assertions_finished_barrier.wait();
        runner.wait_for_jobs().unwrap();

        assert!(!job_exists(job_id, &mut conn));
    }

    #[test]
    fn jobs_are_deleted_when_successfully_run() {
        #[derive(Serialize, Deserialize)]
        struct TestJob;

        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = ();

            fn run(&self, _ctx: &Self::Context) -> anyhow::Result<()> {
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

        runner.run_all_pending_jobs().unwrap();
        runner.wait_for_jobs().unwrap();
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

        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = TestContext;

            fn run(&self, ctx: &Self::Context) -> anyhow::Result<()> {
                ctx.job_started_barrier.wait();
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

        runner.run_all_pending_jobs().unwrap();
        test_context.job_started_barrier.wait();

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

        runner.wait_for_jobs().unwrap();
    }

    #[test]
    fn panicking_in_jobs_updates_retry_counter() {
        #[derive(Serialize, Deserialize)]
        struct TestJob;

        impl BackgroundJob for TestJob {
            const JOB_NAME: &'static str = "test";
            type Context = ();

            fn run(&self, _ctx: &Self::Context) -> anyhow::Result<()> {
                panic!()
            }
        }

        let test_database = TestDatabase::new();

        let runner = runner(test_database.url(), ()).register_job_type::<TestJob>();

        let job_id = TestJob.enqueue(&mut runner.connection().unwrap()).unwrap();

        runner.run_all_pending_jobs().unwrap();
        runner.wait_for_jobs().unwrap();

        let tries = background_jobs::table
            .find(job_id)
            .select(background_jobs::retries)
            .for_update()
            .first::<i32>(&mut *runner.connection().unwrap())
            .unwrap();
        assert_eq!(tries, 1);
    }

    fn runner<Context: Clone + Send + 'static>(
        database_url: &str,
        context: Context,
    ) -> Runner<Context> {
        let connection_pool = r2d2::Pool::builder()
            .max_size(4)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(database_url));

        let connection_pool = DieselPool::new_background_worker(connection_pool);

        Runner::new(connection_pool, context)
            .num_workers(2)
            .job_start_timeout(Duration::from_secs(10))
    }
}
