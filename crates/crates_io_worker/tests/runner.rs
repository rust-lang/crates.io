use claims::{assert_none, assert_some};
use crates_io_test_db::TestDatabase;
use crates_io_worker::schema::background_jobs;
use crates_io_worker::{BackgroundJob, Runner};
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use futures_util::StreamExt;
use insta::assert_compact_json_snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tokio::sync::{Barrier, Notify};
use tokio::time::{sleep, timeout};

async fn all_jobs(conn: &mut AsyncPgConnection) -> QueryResult<Vec<(String, Value)>> {
    background_jobs::table
        .select((background_jobs::job_type, background_jobs::data))
        .get_results(conn)
        .await
}

async fn job_exists(id: i64, conn: &mut AsyncPgConnection) -> QueryResult<bool> {
    Ok(background_jobs::table
        .find(id)
        .select(background_jobs::id)
        .get_result::<i64>(conn)
        .await
        .optional()?
        .is_some())
}

async fn job_is_locked(id: i64, conn: &mut AsyncPgConnection) -> QueryResult<bool> {
    Ok(background_jobs::table
        .find(id)
        .select(background_jobs::id)
        .for_update()
        .skip_locked()
        .get_result::<i64>(conn)
        .await
        .optional()?
        .is_none())
}

#[tokio::test]
async fn jobs_are_locked_when_fetched() -> anyhow::Result<()> {
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

    let pool = pool(test_database.url())?;
    let mut conn = pool.get().await?;

    let runner = runner(pool, test_context.clone()).register_job_type::<TestJob>();

    let job_id = assert_some!(TestJob.enqueue(&conn).await?);

    assert!(job_exists(job_id, &mut conn).await?);
    assert!(!job_is_locked(job_id, &mut conn).await?);

    let runner = runner.start();
    test_context.job_started_barrier.wait().await;

    assert!(job_exists(job_id, &mut conn).await?);
    assert!(job_is_locked(job_id, &mut conn).await?);

    test_context.assertions_finished_barrier.wait().await;
    runner.wait_for_shutdown().await;

    assert!(!job_exists(job_id, &mut conn).await?);

    Ok(())
}

#[tokio::test]
async fn jobs_are_deleted_when_successfully_run() -> anyhow::Result<()> {
    #[derive(Serialize, Deserialize)]
    struct TestJob;

    impl BackgroundJob for TestJob {
        const JOB_NAME: &'static str = "test";
        type Context = ();

        async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
            Ok(())
        }
    }

    async fn remaining_jobs(conn: &mut AsyncPgConnection) -> QueryResult<i64> {
        background_jobs::table.count().get_result(conn).await
    }

    let test_database = TestDatabase::new();

    let pool = pool(test_database.url())?;
    let mut conn = pool.get().await?;

    let runner = runner(pool, ()).register_job_type::<TestJob>();

    assert_eq!(remaining_jobs(&mut conn).await?, 0);

    TestJob.enqueue(&conn).await?;
    assert_eq!(remaining_jobs(&mut conn).await?, 1);

    let runner = runner.start();
    runner.wait_for_shutdown().await;
    assert_eq!(remaining_jobs(&mut conn).await?, 0);

    Ok(())
}

#[tokio::test]
async fn failed_jobs_do_not_release_lock_before_updating_retry_time() -> anyhow::Result<()> {
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

    let pool = pool(test_database.url())?;
    let mut conn = pool.get().await?;

    let runner = runner(pool, test_context.clone()).register_job_type::<TestJob>();

    TestJob.enqueue(&conn).await?;

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
        .load::<i64>(&mut conn)
        .await?;
    assert_eq!(available_jobs.len(), 0);

    // Sanity check to make sure the job actually is there
    let total_jobs_including_failed = background_jobs::table
        .select(background_jobs::id)
        .for_update()
        .load::<i64>(&mut conn)
        .await?;
    assert_eq!(total_jobs_including_failed.len(), 1);

    runner.wait_for_shutdown().await;

    Ok(())
}

#[tokio::test]
async fn panicking_in_jobs_updates_retry_counter() -> anyhow::Result<()> {
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

    let pool = pool(test_database.url())?;
    let mut conn = pool.get().await?;

    let runner = runner(pool, ()).register_job_type::<TestJob>();

    let job_id = assert_some!(TestJob.enqueue(&conn).await?);

    let runner = runner.start();
    runner.wait_for_shutdown().await;

    let tries = background_jobs::table
        .find(job_id)
        .select(background_jobs::retries)
        .for_update()
        .first::<i32>(&mut conn)
        .await?;
    assert_eq!(tries, 1);

    Ok(())
}

#[tokio::test]
async fn jobs_can_be_deduplicated() -> anyhow::Result<()> {
    #[derive(Clone)]
    struct TestContext {
        runs: Arc<AtomicU8>,
        job_started_barrier: Arc<Barrier>,
        assertions_finished_barrier: Arc<Barrier>,
    }

    #[derive(Serialize, Deserialize)]
    struct TestJob {
        value: String,
    }

    impl TestJob {
        fn new(value: impl Into<String>) -> Self {
            let value = value.into();
            Self { value }
        }
    }

    impl BackgroundJob for TestJob {
        const JOB_NAME: &'static str = "test";
        const DEDUPLICATED: bool = true;
        type Context = TestContext;

        async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
            let runs = ctx.runs.fetch_add(1, Ordering::SeqCst);
            if runs == 0 {
                ctx.job_started_barrier.wait().await;
                ctx.assertions_finished_barrier.wait().await;
            }
            Ok(())
        }
    }

    let test_database = TestDatabase::new();

    let test_context = TestContext {
        runs: Arc::new(AtomicU8::new(0)),
        job_started_barrier: Arc::new(Barrier::new(2)),
        assertions_finished_barrier: Arc::new(Barrier::new(2)),
    };

    let pool = pool(test_database.url())?;
    let mut conn = pool.get().await?;

    let runner = Runner::new(pool, test_context.clone())
        .register_job_type::<TestJob>()
        .shutdown_when_queue_empty();

    // Enqueue first job
    assert_some!(TestJob::new("foo").enqueue(&conn).await?);
    assert_compact_json_snapshot!(all_jobs(&mut conn).await?, @r#"[["test", {"value": "foo"}]]"#);

    // Try to enqueue the same job again, which should be deduplicated
    assert_none!(TestJob::new("foo").enqueue(&conn).await?);
    assert_compact_json_snapshot!(all_jobs(&mut conn).await?, @r#"[["test", {"value": "foo"}]]"#);

    // Start processing the first job
    let runner = runner.start();
    test_context.job_started_barrier.wait().await;

    // Enqueue the same job again, which should NOT be deduplicated,
    // since the first job already still running
    assert_some!(TestJob::new("foo").enqueue(&conn).await?);
    assert_compact_json_snapshot!(all_jobs(&mut conn).await?, @r#"[["test", {"value": "foo"}], ["test", {"value": "foo"}]]"#);

    // Try to enqueue the same job again, which should be deduplicated again
    assert_none!(TestJob::new("foo").enqueue(&conn).await?);
    assert_compact_json_snapshot!(all_jobs(&mut conn).await?, @r#"[["test", {"value": "foo"}], ["test", {"value": "foo"}]]"#);

    // Enqueue the same job but with different data, which should
    // NOT be deduplicated
    assert_some!(TestJob::new("bar").enqueue(&conn).await?);
    assert_compact_json_snapshot!(all_jobs(&mut conn).await?, @r#"[["test", {"value": "foo"}], ["test", {"value": "foo"}], ["test", {"value": "bar"}]]"#);

    // Resolve the final barrier to finish the test
    test_context.assertions_finished_barrier.wait().await;
    runner.wait_for_shutdown().await;

    Ok(())
}

/// A database trigger should emit a `NOTIFY` on the `background_jobs` channel
/// whenever a job is enqueued, so that listening workers can wake up
/// immediately instead of waiting for the next poll.
#[tokio::test]
async fn enqueueing_a_job_emits_a_notification() -> anyhow::Result<()> {
    #[derive(Serialize, Deserialize)]
    struct TestJob;

    impl BackgroundJob for TestJob {
        const JOB_NAME: &'static str = "test";
        type Context = ();

        async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
            Ok(())
        }
    }

    let test_database = TestDatabase::new();

    // One connection listens on the channel…
    let mut listen_conn = AsyncPgConnection::establish(test_database.url()).await?;
    diesel::sql_query("LISTEN background_jobs")
        .execute(&mut listen_conn)
        .await?;

    // …while another one enqueues a job.
    let conn = AsyncPgConnection::establish(test_database.url()).await?;
    TestJob.enqueue(&conn).await?;

    // The listener should receive a notification on the expected channel.
    let mut notifications = std::pin::pin!(listen_conn.notifications_stream());
    let notification = timeout(Duration::from_secs(5), notifications.next())
        .await?
        .expect("notification stream ended unexpectedly")?;

    assert_eq!(notification.channel, "background_jobs");

    Ok(())
}

/// A worker should be woken by the database notification as soon as a job is
/// enqueued, instead of waiting for the next poll. The poll interval is set far
/// higher than the assertion timeout, so a timely pickup can only be the result
/// of the notification rather than of polling.
///
/// This runs on a multi-threaded runtime because, unlike the other tests, it
/// keeps a non-shutdown runner (with its listener) alive in the background, and
/// relies on the timeout firing independently of those busy tasks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workers_wake_up_on_notification() -> anyhow::Result<()> {
    #[derive(Clone)]
    struct TestContext {
        job_ran: Arc<Notify>,
    }

    #[derive(Serialize, Deserialize)]
    struct TestJob;

    impl BackgroundJob for TestJob {
        const JOB_NAME: &'static str = "test";
        type Context = TestContext;

        async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
            ctx.job_ran.notify_one();
            Ok(())
        }
    }

    let test_database = TestDatabase::new();

    let test_context = TestContext {
        job_ran: Arc::new(Notify::new()),
    };

    let pool = pool(test_database.url())?;
    let conn = pool.get().await?;

    // Note the deliberately long poll interval and the lack of
    // `shutdown_when_queue_empty()`, so that the listener is started and the
    // worker relies on the notification rather than polling.
    let runner = Runner::new(pool, test_context.clone())
        .register_job_type::<TestJob>()
        .configure_default_queue(|queue| queue.poll_interval(Duration::from_secs(3600)));

    let _handle = runner.start();

    // Give the listener a moment to establish its `LISTEN` before enqueueing, so
    // that the notification is not emitted before anyone is listening.
    sleep(Duration::from_secs(1)).await;

    // Enqueue the job only once the worker is idle and waiting for a notification.
    TestJob.enqueue(&conn).await?;

    // The worker should wake up and run the job well within the poll interval.
    timeout(Duration::from_secs(5), test_context.job_ran.notified())
        .await
        .map_err(|_| anyhow::anyhow!("worker was not woken by the notification"))?;

    Ok(())
}

fn pool(database_url: &str) -> anyhow::Result<Pool<AsyncPgConnection>> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Ok(Pool::builder(manager).max_size(4).build()?)
}

fn runner<Context: Clone + Send + Sync + 'static>(
    deadpool: Pool<AsyncPgConnection>,
    context: Context,
) -> Runner<Context> {
    Runner::new(deadpool, context)
        .configure_default_queue(|queue| queue.num_workers(2))
        .shutdown_when_queue_empty()
}
