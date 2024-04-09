use crates_io_test_db::TestDatabase;
use crates_io_worker::schema::background_jobs;
use crates_io_worker::{BackgroundJob, Runner};
use deadpool_diesel::postgres::{Manager, Pool};
use deadpool_diesel::Runtime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::runtime::Handle;
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

    let runner = runner(test_database.url(), test_context.clone()).register_job_type::<TestJob>();

    let mut conn = test_database.connect();
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

    let mut conn = test_database.connect();
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

    let runner = runner(test_database.url(), test_context.clone()).register_job_type::<TestJob>();

    let mut conn = test_database.connect();
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

    impl BackgroundJob for TestJob {
        const JOB_NAME: &'static str = "test";
        type Context = ();

        async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
            panic!()
        }
    }

    let test_database = TestDatabase::new();

    let runner = runner(test_database.url(), ()).register_job_type::<TestJob>();

    let mut conn = test_database.connect();

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

fn runner<Context: Clone + Send + Sync + 'static>(
    database_url: &str,
    context: Context,
) -> Runner<Context> {
    let manager = Manager::new(database_url, Runtime::Tokio1);
    let deadpool = Pool::builder(manager).max_size(4).build().unwrap();

    Runner::new(&Handle::current(), deadpool, context)
        .configure_default_queue(|queue| queue.num_workers(2))
        .shutdown_when_queue_empty()
}
