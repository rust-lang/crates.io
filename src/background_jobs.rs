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
use crate::worker::{
    DailyDbMaintenanceJob, DumpDbJob, NormalizeIndexJob, RenderAndUploadReadmeJob, SquashIndexJob,
    SyncToGitIndexJob, SyncToSparseIndexJob, UpdateDownloadsJob,
};
use crates_io_index::Repository;

pub const PRIORITY_DEFAULT: i16 = 0;
pub const PRIORITY_RENDER_README: i16 = 50;
pub const PRIORITY_SYNC_TO_INDEX: i16 = 100;

pub trait BackgroundJob: Serialize + DeserializeOwned + 'static {
    /// Unique name of the task.
    ///
    /// This MUST be unique for the whole application.
    const JOB_NAME: &'static str;

    /// Execute the task. This method should define its logic
    fn run(&self, state: PerformState<'_>, env: &Environment) -> Result<(), PerformError>;

    fn enqueue(&self, conn: &mut PgConnection) -> Result<(), EnqueueError> {
        self.enqueue_with_priority(conn, PRIORITY_DEFAULT)
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

macro_rules! jobs {
    {
        $vis:vis enum $name:ident {
            $($variant:ident ($content:ident)),+ $(,)?
        }
    } => {
        $vis enum $name {
            $($variant ($content),)+
        }

        impl $name {
            pub fn from_value(job_type: &str, value: serde_json::Value) -> Result<Self, PerformError> {
                Ok(match job_type {
                    $($content::JOB_NAME => Self::$variant(serde_json::from_value(value)?),)+
                    job_type => Err(PerformError::from(format!("Unknown job type {job_type}")))?,
                })
            }

            pub(super) fn perform(
                &self,
                env: &Option<Environment>,
                state: PerformState<'_>,
            ) -> Result<(), PerformError> {
                let env = env
                    .as_ref()
                    .expect("Application should configure a background runner environment");
                match self {
                    $(Self::$variant(job) => job.run(state, env),)+
                }
            }
        }
    }
}

jobs! {
    pub enum Job {
        DailyDbMaintenance(DailyDbMaintenanceJob),
        DumpDb(DumpDbJob),
        NormalizeIndex(NormalizeIndexJob),
        RenderAndUploadReadme(RenderAndUploadReadmeJob),
        SquashIndex(SquashIndexJob),
        SyncToGitIndex(SyncToGitIndexJob),
        SyncToSparseIndex(SyncToSparseIndexJob),
        UpdateDownloads(UpdateDownloadsJob),
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

impl Job {
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
        let find_similar_jobs_query = |job_type: &'static str, data: serde_json::Value| {
            background_jobs::table
                .select(background_jobs::id)
                .filter(background_jobs::job_type.eq(job_type))
                .filter(background_jobs::data.eq(data))
                .filter(background_jobs::priority.eq(PRIORITY_SYNC_TO_INDEX))
                .for_update()
                .skip_locked()
        };

        // Returns one `job_type, data, priority` row with values from the
        // passed-in `job`, unless a similar row already exists.
        let deduplicated_select_query = |job_type: &'static str, data: serde_json::Value| {
            diesel::select((
                job_type.into_sql::<Text>(),
                data.clone().into_sql::<Jsonb>(),
                PRIORITY_SYNC_TO_INDEX.into_sql::<Int2>(),
            ))
            .filter(not(exists(find_similar_jobs_query(job_type, data))))
        };

        let to_git = serde_json::to_value(SyncToGitIndexJob::new(krate.to_string()))?;
        let to_git = deduplicated_select_query(SyncToGitIndexJob::JOB_NAME, to_git);

        let to_sparse = serde_json::to_value(SyncToSparseIndexJob::new(krate.to_string()))?;
        let to_sparse = deduplicated_select_query(SyncToSparseIndexJob::JOB_NAME, to_sparse);

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
