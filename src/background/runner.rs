#![allow(dead_code)]
use diesel::prelude::*;
use std::any::Any;
use std::panic::{catch_unwind, PanicInfo, RefUnwindSafe, UnwindSafe};
use std::sync::Arc;
use threadpool::ThreadPool;

use super::{storage, Job, Registry};
use crate::db::{DieselPool, DieselPooledConn};
use crate::util::errors::*;

#[allow(missing_debug_implementations)]
pub struct Builder<Env> {
    connection_pool: DieselPool,
    environment: Env,
    registry: Registry<Env>,
    thread_count: Option<usize>,
}

impl<Env> Builder<Env> {
    pub fn register<T: Job<Environment = Env>>(mut self) -> Self {
        self.registry.register::<T>();
        self
    }

    pub fn thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = Some(thread_count);
        self
    }

    pub fn build(self) -> Runner<Env> {
        Runner {
            connection_pool: self.connection_pool,
            thread_pool: ThreadPool::new(self.thread_count.unwrap_or(5)),
            environment: Arc::new(self.environment),
            registry: Arc::new(self.registry),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct Runner<Env> {
    connection_pool: DieselPool,
    thread_pool: ThreadPool,
    environment: Arc<Env>,
    registry: Arc<Registry<Env>>,
}

impl<Env: RefUnwindSafe + Send + Sync + 'static> Runner<Env> {
    pub fn builder(connection_pool: DieselPool, environment: Env) -> Builder<Env> {
        Builder {
            connection_pool,
            environment,
            registry: Registry::new(),
            thread_count: None,
        }
    }

    pub fn run_all_pending_jobs(&self) -> CargoResult<()> {
        let available_job_count = storage::available_job_count(&*self.connection()?)?;
        for _ in 0..available_job_count {
            self.run_single_job()
        }
        Ok(())
    }

    fn run_single_job(&self) {
        let environment = Arc::clone(&self.environment);
        let registry = Arc::clone(&self.registry);
        self.get_single_job(move |job| {
            let perform_fn = registry
                .get(&job.job_type)
                .ok_or_else(|| internal(&format_args!("Unknown job type {}", job.job_type)))?;
            perform_fn(job.data, &environment)
        })
    }

    fn get_single_job<F>(&self, f: F)
    where
        F: FnOnce(storage::BackgroundJob) -> CargoResult<()> + Send + UnwindSafe + 'static,
    {
        let conn = self.connection().expect("Could not acquire connection");
        self.thread_pool.execute(move || {
            conn.transaction::<_, Box<dyn CargoError>, _>(|| {
                let job = storage::find_next_unlocked_job(&conn).optional()?;
                let job = match job {
                    Some(j) => j,
                    None => return Ok(()),
                };
                let job_id = job.id;

                let result = catch_unwind(|| f(job))
                    .map_err(|e| try_to_extract_panic_info(&e))
                    .and_then(|r| r);

                match result {
                    Ok(_) => storage::delete_successful_job(&conn, job_id)?,
                    Err(e) => {
                        eprintln!("Job {} failed to run: {}", job_id, e);
                        storage::update_failed_job(&conn, job_id);
                    }
                }
                Ok(())
            })
            .expect("Could not retrieve or update job")
        })
    }

    fn connection(&self) -> CargoResult<DieselPooledConn> {
        self.connection_pool.get().map_err(Into::into)
    }

    pub fn assert_no_failed_jobs(&self) -> CargoResult<()> {
        self.wait_for_jobs();
        let failed_jobs = storage::failed_job_count(&*self.connection()?)?;
        assert_eq!(0, failed_jobs);
        Ok(())
    }

    fn wait_for_jobs(&self) {
        self.thread_pool.join();
        assert_eq!(0, self.thread_pool.panic_count());
    }
}

/// Try to figure out what's in the box, and print it if we can.
///
/// The actual error type we will get from `panic::catch_unwind` is really poorly documented.
/// However, the `panic::set_hook` functions deal with a `PanicInfo` type, and its payload is
/// documented as "commonly but not always `&'static str` or `String`". So we can try all of those,
/// and give up if we didn't get one of those three types.
fn try_to_extract_panic_info(info: &(dyn Any + Send + 'static)) -> Box<dyn CargoError> {
    if let Some(x) = info.downcast_ref::<PanicInfo>() {
        internal(&format_args!("job panicked: {}", x))
    } else if let Some(x) = info.downcast_ref::<&'static str>() {
        internal(&format_args!("job panicked: {}", x))
    } else if let Some(x) = info.downcast_ref::<String>() {
        internal(&format_args!("job panicked: {}", x))
    } else {
        internal("job panicked")
    }
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;
    use diesel::r2d2;

    use super::*;
    use crate::schema::background_jobs::dsl::*;
    use std::panic::AssertUnwindSafe;
    use std::sync::{Arc, Barrier, Mutex, MutexGuard};

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

        runner.get_single_job(move |job| {
            fetch_barrier.0.wait(); // Tell thread 2 it can lock its job
            assert_eq!(first_job_id, job.id);
            return_barrier.0.wait(); // Wait for thread 2 to lock its job
            Ok(())
        });

        fetch_barrier2.0.wait(); // Wait until thread 1 locks its job
        runner.get_single_job(move |job| {
            assert_eq!(second_job_id, job.id);
            return_barrier2.0.wait(); // Tell thread 1 it can unlock its job
            Ok(())
        });

        runner.wait_for_jobs();
    }

    #[test]
    fn jobs_are_deleted_when_successfully_run() {
        let _guard = TestGuard::lock();

        let runner = runner();
        create_dummy_job(&runner);

        runner.get_single_job(|_| Ok(()));
        runner.wait_for_jobs();

        let remaining_jobs = background_jobs
            .count()
            .get_result(&runner.connection().unwrap());
        assert_eq!(Ok(0), remaining_jobs);
    }

    #[test]
    fn failed_jobs_do_not_release_lock_before_updating_retry_time() {
        let _guard = TestGuard::lock();

        let runner = runner();
        create_dummy_job(&runner);
        let barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let barrier2 = barrier.clone();

        runner.get_single_job(move |_| {
            barrier.0.wait();
            // error so the job goes back into the queue
            Err(human("nope"))
        });

        let conn = runner.connection().unwrap();
        // Wait for the first thread to acquire the lock
        barrier2.0.wait();
        // We are intentionally not using `get_single_job` here.
        // `SKIP LOCKED` is intentionally omitted here, so we block until
        // the lock on the first job is released.
        // If there is any point where the row is unlocked, but the retry
        // count is not updated, we will get a row here.
        let available_jobs = background_jobs
            .select(id)
            .filter(retries.eq(0))
            .for_update()
            .load::<i64>(&conn)
            .unwrap();
        assert_eq!(0, available_jobs.len());

        // Sanity check to make sure the job actually is there
        let total_jobs_including_failed = background_jobs
            .select(id)
            .for_update()
            .load::<i64>(&conn)
            .unwrap();
        assert_eq!(1, total_jobs_including_failed.len());

        runner.wait_for_jobs();
    }

    #[test]
    fn panicking_in_jobs_updates_retry_counter() {
        let _guard = TestGuard::lock();
        let runner = runner();
        let job_id = create_dummy_job(&runner).id;

        runner.get_single_job(|_| panic!());
        runner.wait_for_jobs();

        let tries = background_jobs
            .find(job_id)
            .select(retries)
            .for_update()
            .first::<i32>(&runner.connection().unwrap())
            .unwrap();
        assert_eq!(1, tries);
    }

    lazy_static! {
        // Since these tests deal with behavior concerning multiple connections
        // running concurrently, they have to run outside of a transaction.
        // Therefore we can't run more than one at a time.
        //
        // Rather than forcing the whole suite to be run with `--test-threads 1`,
        // we just lock these tests instead.
        static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
    }

    struct TestGuard<'a>(MutexGuard<'a, ()>);

    impl<'a> TestGuard<'a> {
        fn lock() -> Self {
            TestGuard(TEST_MUTEX.lock().unwrap())
        }
    }

    impl<'a> Drop for TestGuard<'a> {
        fn drop(&mut self) {
            ::diesel::sql_query("TRUNCATE TABLE background_jobs")
                .execute(&runner().connection().unwrap())
                .unwrap();
        }
    }

    fn runner() -> Runner<()> {
        use dotenv;

        let database_url =
            dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
        let manager = r2d2::ConnectionManager::new(database_url);
        let pool = r2d2::Pool::builder().max_size(2).build(manager).unwrap();

        Runner::builder(pool, ()).thread_count(2).build()
    }

    fn create_dummy_job(runner: &Runner<()>) -> storage::BackgroundJob {
        ::diesel::insert_into(background_jobs)
            .values((job_type.eq("Foo"), data.eq(json!(null))))
            .returning((id, job_type, data))
            .get_result(&runner.connection().unwrap())
            .unwrap()
    }
}
