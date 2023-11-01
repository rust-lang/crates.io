use diesel::dsl::{exists, not};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sql_types::{Int2, Jsonb, Text};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Display;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::db::ConnectionPool;
use crate::schema::background_jobs;
use crate::storage::Storage;
use crate::swirl::errors::EnqueueError;
use crate::swirl::PerformError;
use crate::worker::cloudfront::CloudFront;
use crate::worker::fastly::Fastly;
use crate::worker::{SyncToGitIndexJob, SyncToSparseIndexJob};
use crates_io_index::Repository;

pub trait BackgroundJob: Serialize + DeserializeOwned + 'static {
    /// Unique name of the task.
    ///
    /// This MUST be unique for the whole application.
    const JOB_NAME: &'static str;

    /// Default priority of the task.
    ///
    /// [Self::enqueue_with_priority] can be used to override the priority value.
    const PRIORITY: i16 = 0;

    /// The application data provided to this job at runtime.
    type Context: Clone + Send + 'static;

    /// Execute the task. This method should define its logic
    fn run(&self, state: PerformState<'_>, env: &Self::Context) -> Result<(), PerformError>;

    fn enqueue(&self, conn: &mut PgConnection) -> Result<(), EnqueueError> {
        self.enqueue_with_priority(conn, Self::PRIORITY)
    }

    #[instrument(name = "swirl.enqueue", skip(self, conn), fields(message = Self::JOB_NAME))]
    fn enqueue_with_priority(
        &self,
        conn: &mut PgConnection,
        job_priority: i16,
    ) -> Result<(), EnqueueError> {
        let job_data = serde_json::to_value(self)?;
        diesel::insert_into(background_jobs::table)
            .values((
                background_jobs::job_type.eq(Self::JOB_NAME),
                background_jobs::data.eq(job_data),
                background_jobs::priority.eq(job_priority),
            ))
            .execute(conn)?;
        Ok(())
    }
}

/// Database state that is passed to `Job::perform()`.
pub struct PerformState<'a> {
    /// The existing connection used to lock the background job.
    ///
    /// Most jobs can reuse the existing connection, however it will already be within a
    /// transaction and is thus not appropriate in all cases.
    pub(crate) conn: &'a mut PgConnection,
    /// A connection pool for obtaining a unique connection.
    ///
    /// This will be `None` within our standard test framework, as there everything is expected to
    /// run within a single transaction.
    pub(crate) pool: Option<ConnectionPool>,
}

impl PerformState<'_> {
    /// A helper function for jobs needing a fresh connection (i.e. not already within a transaction).
    ///
    /// This will error when run from our main test framework, as there most work is expected to be
    /// done within an existing transaction.
    pub fn fresh_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, PerformError> {
        match self.pool {
            // In production a pool should be available. This can only be hit in tests, which don't
            // provide the pool.
            None => Err(String::from("Database pool was unavailable").into()),
            Some(ref pool) => Ok(pool.get()?),
        }
    }
}

/// Enqueue both index sync jobs (git and sparse) for a crate, unless they
/// already exist in the background job queue.
///
/// Note that there are currently no explicit tests for this functionality,
/// since our test suite only allows us to use a single database connection
/// and the background worker queue locking only work when using multiple
/// connections.
#[instrument(name = "swirl.enqueue", skip_all, fields(message = "sync_to_index", krate = %krate))]
pub fn enqueue_sync_to_index<T: Display>(
    krate: T,
    conn: &mut PgConnection,
) -> Result<(), EnqueueError> {
    // Returns jobs with matching `job_type`, `data` and `priority`,
    // skipping ones that are already locked by the background worker.
    let find_similar_jobs_query =
        |job_type: &'static str, data: serde_json::Value, priority: i16| {
            background_jobs::table
                .select(background_jobs::id)
                .filter(background_jobs::job_type.eq(job_type))
                .filter(background_jobs::data.eq(data))
                .filter(background_jobs::priority.eq(priority))
                .for_update()
                .skip_locked()
        };

    // Returns one `job_type, data, priority` row with values from the
    // passed-in `job`, unless a similar row already exists.
    let deduplicated_select_query =
        |job_type: &'static str, data: serde_json::Value, priority: i16| {
            diesel::select((
                job_type.into_sql::<Text>(),
                data.clone().into_sql::<Jsonb>(),
                priority.into_sql::<Int2>(),
            ))
            .filter(not(exists(find_similar_jobs_query(
                job_type, data, priority,
            ))))
        };

    let to_git = deduplicated_select_query(
        SyncToGitIndexJob::JOB_NAME,
        serde_json::to_value(SyncToGitIndexJob::new(krate.to_string()))?,
        SyncToGitIndexJob::PRIORITY,
    );

    let to_sparse = deduplicated_select_query(
        SyncToSparseIndexJob::JOB_NAME,
        serde_json::to_value(SyncToSparseIndexJob::new(krate.to_string()))?,
        SyncToSparseIndexJob::PRIORITY,
    );

    // Insert index update background jobs, but only if they do not
    // already exist.
    let added_jobs_count = diesel::insert_into(background_jobs::table)
        .values(to_git.union_all(to_sparse))
        .into_columns((
            background_jobs::job_type,
            background_jobs::data,
            background_jobs::priority,
        ))
        .execute(conn)?;

    // Print a log event if we skipped inserting a job due to deduplication.
    if added_jobs_count != 2 {
        let skipped_jobs_count = 2 - added_jobs_count;
        info!(%skipped_jobs_count, "Skipped adding duplicate jobs to the background worker queue");
    }

    Ok(())
}

pub struct Environment {
    index: Mutex<Repository>,
    http_client: AssertUnwindSafe<Client>,
    cloudfront: Option<CloudFront>,
    fastly: Option<Fastly>,
    pub storage: AssertUnwindSafe<Arc<Storage>>,
}

impl Environment {
    pub fn new(
        index: Repository,
        http_client: Client,
        cloudfront: Option<CloudFront>,
        fastly: Option<Fastly>,
        storage: Arc<Storage>,
    ) -> Self {
        Self {
            index: Mutex::new(index),
            http_client: AssertUnwindSafe(http_client),
            cloudfront,
            fastly,
            storage: AssertUnwindSafe(storage),
        }
    }

    #[instrument(skip_all)]
    pub fn lock_index(&self) -> Result<MutexGuard<'_, Repository>, PerformError> {
        let repo = self.index.lock().unwrap_or_else(PoisonError::into_inner);
        repo.reset_head()?;
        Ok(repo)
    }

    /// Returns a client for making HTTP requests to upload crate files.
    pub(crate) fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub(crate) fn cloudfront(&self) -> Option<&CloudFront> {
        self.cloudfront.as_ref()
    }

    pub(crate) fn fastly(&self) -> Option<&Fastly> {
        self.fastly.as_ref()
    }
}
