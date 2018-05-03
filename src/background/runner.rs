#![allow(dead_code)]
use diesel::prelude::*;
use std::panic::{catch_unwind, UnwindSafe};

use super::storage;
use util::errors::*;

fn get_single_job<F>(conn: &PgConnection, f: F) -> CargoResult<()>
where
    F: FnOnce(storage::BackgroundJob) -> CargoResult<()> + UnwindSafe,
{
    conn.transaction::<_, Box<dyn CargoError>, _>(|| {
        let job = storage::find_next_unlocked_job(conn)?;
        let job_id = job.id;

        let result = catch_unwind(|| f(job))
            .map_err(|_| internal("job panicked"))
            .and_then(|r| r);

        if result.is_ok() {
            storage::delete_successful_job(conn, job_id)?;
        } else {
            storage::update_failed_job(conn, job_id);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use diesel::prelude::*;

    use schema::background_jobs::dsl::*;
    use std::sync::{Mutex, MutexGuard, Barrier, Arc};
    use std::panic::AssertUnwindSafe;
    use std::thread;
    use super::*;

    #[test]
    fn jobs_are_locked_when_fetched() {
        let _guard = TestGuard::lock();

        let conn = connection();
        let first_job_id = create_dummy_job(&conn).id;
        let second_job_id = create_dummy_job(&conn).id;
        let fetch_barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let fetch_barrier2 = fetch_barrier.clone();
        let return_barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let return_barrier2 = return_barrier.clone();

        let t1 = thread::spawn(move || {
            let _ = get_single_job(&connection(), |job| {
                fetch_barrier.0.wait(); // Tell thread 2 it can lock its job
                assert_eq!(first_job_id, job.id);
                return_barrier.0.wait(); // Wait for thread 2 to lock its job
                Ok(())
            });
        });

        let t2 = thread::spawn(move || {
            fetch_barrier2.0.wait(); // Wait until thread 1 locks its job
            get_single_job(&connection(), |job| {
                assert_eq!(second_job_id, job.id);
                return_barrier2.0.wait(); // Tell thread 1 it can unlock its job
                Ok(())
            })
            .unwrap();
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn jobs_are_deleted_when_successfully_run() {
        let _guard = TestGuard::lock();

        let conn = connection();
        create_dummy_job(&conn);

        get_single_job(&conn, |_| {
            Ok(())
        }).unwrap();

        let remaining_jobs = background_jobs.count()
            .get_result(&conn);
        assert_eq!(Ok(0), remaining_jobs);
    }

    #[test]
    fn failed_jobs_do_not_release_lock_before_updating_retry_time() {
        let _guard = TestGuard::lock();
        create_dummy_job(&connection());
        let barrier = Arc::new(AssertUnwindSafe(Barrier::new(2)));
        let barrier2 = barrier.clone();

        let t1 = thread::spawn(move || {
            let _ = get_single_job(&connection(), |_| {
                barrier.0.wait();
                // error so the job goes back into the queue
                Err(human("nope"))
            });
        });

        let t2 = thread::spawn(move || {
            let conn = connection();
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
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn panicking_in_jobs_updates_retry_counter() {
        let _guard = TestGuard::lock();
        let conn = connection();
        let job_id = create_dummy_job(&conn).id;

        let t1 = thread::spawn(move || {
            let _ = get_single_job(&connection(), |_| {
                panic!()
            });
        });

        let _ = t1.join();

        let tries = background_jobs
            .find(job_id)
            .select(retries)
            .for_update()
            .first::<i32>(&conn)
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
                .execute(&connection())
                .unwrap();
        }
    }

    fn connection() -> PgConnection {
        use dotenv;

        let database_url =
            dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
        PgConnection::establish(&database_url).unwrap()
    }

    fn create_dummy_job(conn: &PgConnection) -> storage::BackgroundJob {
        ::diesel::insert_into(background_jobs)
            .values((job_type.eq("Foo"), data.eq(json!(null))))
            .returning((id, job_type, data))
            .get_result(conn)
            .unwrap()
    }
}
