use diesel::connection::{AnsiTransactionManager, TransactionManager};
use diesel::prelude::*;
use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::panic::{catch_unwind, AssertUnwindSafe, PanicInfo, UnwindSafe};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::time::Duration;
use threadpool::ThreadPool;

use super::errors::*;
use super::storage;
use crate::background_jobs::{BackgroundJob, Environment, PerformState};
use crate::db::{DieselPool, DieselPooledConn};
use event::Event;

mod event;

const DEFAULT_JOB_START_TIMEOUT: Duration = Duration::from_secs(30);

type RunTaskFn = Arc<
    dyn Fn(&Environment, PerformState<'_>, serde_json::Value) -> Result<(), PerformError>
        + Send
        + Sync,
>;

fn runnable<J: BackgroundJob>(
    env: &Environment,
    state: PerformState<'_>,
    payload: serde_json::Value,
) -> Result<(), PerformError> {
    let job: J = serde_json::from_value(payload)?;
    job.run(state, env)
}

/// The core runner responsible for locking and running jobs
pub struct Runner {
    connection_pool: DieselPool,
    thread_pool: ThreadPool,
    job_registry: Arc<RwLock<HashMap<String, RunTaskFn>>>,
    environment: Arc<Option<Environment>>,
    job_start_timeout: Duration,
}

impl Runner {
    pub fn new(connection_pool: DieselPool, environment: Arc<Option<Environment>>) -> Self {
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

    pub fn register_job_type<J: BackgroundJob>(self) -> Self {
        self.job_registry
            .write()
            .insert(J::JOB_NAME.to_string(), Arc::new(runnable::<J>));

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
                self.run_single_job(sender.clone());
            }

            pending_messages += jobs_to_queue;
            match receiver.recv_timeout(self.job_start_timeout) {
                Ok(Event::Working) => pending_messages -= 1,
                Ok(Event::NoJobAvailable) => return Ok(()),
                Ok(Event::ErrorLoadingJob(e)) => return Err(FetchError::FailedLoadingJob(e)),
                Ok(Event::FailedToAcquireConnection(e)) => {
                    return Err(FetchError::NoDatabaseConnection(e));
                }
                Err(_) => return Err(FetchError::NoMessageReceived),
            }
        }
    }

    fn run_single_job(&self, sender: SyncSender<Event>) {
        let job_registry = AssertUnwindSafe(self.job_registry.clone());
        let environment = self.environment.clone();
        self.get_single_job(sender, move |job, state| {
            let job_registry = job_registry.read();
            let run_task_fn = job_registry
                .get(&job.job_type)
                .ok_or_else(|| PerformError::from(format!("Unknown job type {}", job.job_type)))?;

            let environment = environment
                .as_ref()
                .as_ref()
                .expect("Application should configure a background runner environment");

            run_task_fn(environment, state, job.data)
        })
    }

    fn get_single_job<F>(&self, sender: SyncSender<Event>, f: F)
    where
        F: FnOnce(storage::BackgroundJob, PerformState<'_>) -> Result<(), PerformError>
            + Send
            + UnwindSafe
            + 'static,
    {
        use diesel::result::Error::RollbackTransaction;

        // The connection may not be `Send` so we need to clone the pool instead
        let pool = self.connection_pool.clone();
        self.thread_pool.execute(move || {
            let conn = &mut *match pool.get() {
                Ok(conn) => conn,
                Err(e) => {
                    // TODO: Review error handling and possibly drop all usage of `let _ =` in this file
                    let _ = sender.send(Event::FailedToAcquireConnection(e));
                    return;
                }
            };

            let job_run_result = conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let job = match storage::find_next_unlocked_job(conn).optional() {
                    Ok(Some(j)) => {
                        let _ = sender.send(Event::Working);
                        j
                    }
                    Ok(None) => {
                        let _ = sender.send(Event::NoJobAvailable);
                        return Ok(());
                    }
                    Err(e) => {
                        let _ = sender.send(Event::ErrorLoadingJob(e));
                        return Err(RollbackTransaction);
                    }
                };
                let job_id = job.id;

                let initial_depth = get_transaction_depth(conn)?;
                if initial_depth != 1 {
                    warn!("Initial transaction depth is not 1. This is very unexpected");
                }

                let tx_ctx = sentry::TransactionContext::new(&job.job_type, "swirl.perform");
                let tx = sentry::start_transaction(tx_ctx);

                let result = sentry::with_scope(
                    |scope| scope.set_span(Some(tx.clone().into())),
                    || {
                        conn.transaction(|conn| {
                            let pool = pool.to_real_pool();
                            let state = AssertUnwindSafe(PerformState { conn, pool });
                            catch_unwind(|| {
                                // Ensure the whole `AssertUnwindSafe(_)` is moved
                                let state = state;
                                f(job, state.0)
                            })
                            .map_err(|e| try_to_extract_panic_info(&e))
                        })
                        // TODO: Replace with flatten() once that stabilizes
                        .and_then(std::convert::identity)
                    },
                );

                tx.set_status(match result.is_ok() {
                    true => sentry::protocol::SpanStatus::Ok,
                    false => sentry::protocol::SpanStatus::UnknownError,
                });
                tx.finish();

                // If the job panics it could leave the connection inside an inner transaction(s).
                // Attempt to roll those back so we can mark the job as failed, but if the rollback
                // fails then there isn't much we can do at this point so return early. `r2d2` will
                // detect the bad state and drop it from the pool.
                loop {
                    let depth = get_transaction_depth(conn)?;
                    if depth == initial_depth {
                        break;
                    }
                    warn!("Rolling back a transaction due to a panic in a background task");
                    AnsiTransactionManager::rollback_transaction(conn)?;
                }

                match result {
                    Ok(_) => storage::delete_successful_job(conn, job_id)?,
                    Err(e) => {
                        eprintln!("Job {job_id} failed to run: {e}");
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
        })
    }

    fn connection(&self) -> Result<DieselPooledConn<'_>, Box<dyn Error + Send + Sync>> {
        self.connection_pool.get().map_err(Into::into)
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
    pub fn check_for_failed_jobs(&self) -> Result<(), FailedJobsError> {
        self.wait_for_jobs()?;
        let failed_jobs = storage::failed_job_count(&mut *self.connection()?)?;
        if failed_jobs == 0 {
            Ok(())
        } else {
            Err(JobsFailed(failed_jobs))
        }
    }

    fn wait_for_jobs(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.thread_pool.join();
        let panic_count = self.thread_pool.panic_count();
        if panic_count == 0 {
            Ok(())
        } else {
            Err(format!("{panic_count} threads panicked").into())
        }
    }
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
fn try_to_extract_panic_info(info: &(dyn Any + Send + 'static)) -> PerformError {
    if let Some(x) = info.downcast_ref::<PanicInfo<'_>>() {
        format!("job panicked: {x}").into()
    } else if let Some(x) = info.downcast_ref::<&'static str>() {
        format!("job panicked: {x}").into()
    } else if let Some(x) = info.downcast_ref::<String>() {
        format!("job panicked: {x}").into()
    } else {
        "job panicked".into()
    }
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;
    use once_cell::sync::Lazy;

    use super::*;
    use crate::schema::background_jobs;
    use diesel::r2d2;
    use diesel::r2d2::ConnectionManager;
    use std::panic::AssertUnwindSafe;
    use std::sync::mpsc::{sync_channel, SyncSender};
    use std::sync::{Arc, Barrier, Mutex, MutexGuard};

    fn dummy_sender<T>() -> SyncSender<T> {
        sync_channel(1).0
    }

    #[test]
    fn jobs_are_locked_when_fetched() {
        let _guard = TestGuard::lock();

        let runner = runner();
        let first_job_id = create_dummy_job(&runner).id;
        let second_job_id = create_dummy_job(&runner).id;
        let fetch_barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let fetch_barrier2 = fetch_barrier.clone();
        let return_barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let return_barrier2 = return_barrier.clone();

        runner.get_single_job(dummy_sender(), move |job, _| {
            fetch_barrier.0.wait(); // Tell thread 2 it can lock its job
            assert_eq!(first_job_id, job.id);
            return_barrier.0.wait(); // Wait for thread 2 to lock its job
            Ok(())
        });

        fetch_barrier2.0.wait(); // Wait until thread 1 locks its job
        runner.get_single_job(dummy_sender(), move |job, _| {
            assert_eq!(second_job_id, job.id);
            return_barrier2.0.wait(); // Tell thread 1 it can unlock its job
            Ok(())
        });

        runner.wait_for_jobs().unwrap();
    }

    #[test]
    fn jobs_are_deleted_when_successfully_run() {
        let _guard = TestGuard::lock();

        let runner = runner();
        create_dummy_job(&runner);

        runner.get_single_job(dummy_sender(), |_, _| Ok(()));
        runner.wait_for_jobs().unwrap();

        let remaining_jobs = background_jobs::table
            .count()
            .get_result(&mut *runner.connection().unwrap());
        assert_eq!(remaining_jobs, Ok(0));
    }

    #[test]
    fn failed_jobs_do_not_release_lock_before_updating_retry_time() {
        let _guard = TestGuard::lock();

        let runner = runner();
        create_dummy_job(&runner);
        let barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let barrier2 = barrier.clone();

        runner.get_single_job(dummy_sender(), move |_, state| {
            state.conn.transaction(|_| {
                barrier.0.wait();
                // The job should go back into the queue after a panic
                panic!();
            })
        });

        let conn = &mut *runner.connection().unwrap();
        // Wait for the first thread to acquire the lock
        barrier2.0.wait();
        // We are intentionally not using `get_single_job` here.
        // `SKIP LOCKED` is intentionally omitted here, so we block until
        // the lock on the first job is released.
        // If there is any point where the row is unlocked, but the retry
        // count is not updated, we will get a row here.
        let available_jobs = background_jobs::table
            .select(background_jobs::id)
            .filter(background_jobs::retries.eq(0))
            .for_update()
            .load::<i64>(conn)
            .unwrap();
        assert_eq!(available_jobs.len(), 0);

        // Sanity check to make sure the job actually is there
        let total_jobs_including_failed = background_jobs::table
            .select(background_jobs::id)
            .for_update()
            .load::<i64>(conn)
            .unwrap();
        assert_eq!(total_jobs_including_failed.len(), 1);

        runner.wait_for_jobs().unwrap();
    }

    #[test]
    fn panicking_in_jobs_updates_retry_counter() {
        let _guard = TestGuard::lock();
        let runner = runner();
        let job_id = create_dummy_job(&runner).id;

        runner.get_single_job(dummy_sender(), |_, _| panic!());
        runner.wait_for_jobs().unwrap();

        let tries = background_jobs::table
            .find(job_id)
            .select(background_jobs::retries)
            .for_update()
            .first::<i32>(&mut *runner.connection().unwrap())
            .unwrap();
        assert_eq!(tries, 1);
    }

    // Since these tests deal with behavior concerning multiple connections
    // running concurrently, they have to run outside of a transaction.
    // Therefore we can't run more than one at a time.
    //
    // Rather than forcing the whole suite to be run with `--test-threads 1`,
    // we just lock these tests instead.
    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct TestGuard<'a>(MutexGuard<'a, ()>);

    impl<'a> TestGuard<'a> {
        fn lock() -> Self {
            TestGuard(TEST_MUTEX.lock().unwrap())
        }
    }

    impl<'a> Drop for TestGuard<'a> {
        fn drop(&mut self) {
            diesel::sql_query("TRUNCATE TABLE background_jobs")
                .execute(&mut *runner().connection().unwrap())
                .unwrap();
        }
    }

    fn runner() -> Runner {
        let database_url =
            dotenvy::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");

        let connection_pool = r2d2::Pool::builder()
            .max_size(4)
            .build_unchecked(ConnectionManager::new(database_url));

        let connection_pool = DieselPool::new_background_worker(connection_pool);

        Runner::new(connection_pool, Arc::new(None))
            .num_workers(2)
            .job_start_timeout(Duration::from_secs(10))
    }

    fn create_dummy_job(runner: &Runner) -> storage::BackgroundJob {
        diesel::insert_into(background_jobs::table)
            .values((
                background_jobs::job_type.eq("Foo"),
                background_jobs::data.eq(serde_json::json!(null)),
            ))
            .returning((
                background_jobs::id,
                background_jobs::job_type,
                background_jobs::data,
            ))
            .get_result(&mut *runner.connection().unwrap())
            .unwrap()
    }
}
