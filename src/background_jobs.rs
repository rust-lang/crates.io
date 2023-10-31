use diesel::dsl::{exists, not};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sql_types::{Int2, Jsonb, Text};
use paste::paste;
use reqwest::blocking::Client;
use std::fmt::Display;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::db::ConnectionPool;
use crate::schema::background_jobs;
use crate::storage::Storage;
use crate::swirl::errors::EnqueueError;
use crate::swirl::PerformError;
use crate::worker;
use crate::worker::cloudfront::CloudFront;
use crate::worker::fastly::Fastly;
use crates_io_index::Repository;

pub const PRIORITY_DEFAULT: i16 = 0;
pub const PRIORITY_RENDER_README: i16 = 50;
pub const PRIORITY_SYNC_TO_INDEX: i16 = 100;

macro_rules! jobs {
    {
        $vis:vis enum $name:ident {
            $($variant:ident $(($content:ident))?),+ $(,)?
        }
    } => {
        $vis enum $name {
            $($variant $(($content))?,)+
        }

        paste! {
            impl $name {
                fn as_type_str(&self) -> &'static str {
                    match self {
                        $(Self:: $variant $(([<_ $content:snake:lower>]))? => stringify!([<$variant:snake:lower>]),)+
                    }
                }

                fn to_value(&self) -> serde_json::Result<serde_json::Value> {
                    match self {
                        $(Self:: $variant $(([<$content:snake:lower>]))? => job_variant_to_value!($([<$content:snake:lower>])?),)+
                    }
                }

                pub fn from_value(job_type: &str, value: serde_json::Value) -> Result<Self, PerformError> {
                    Ok(match job_type {
                        $(stringify!([<$variant:snake:lower>]) => job_variant_from_value!($variant value $($content)?),)+
                        job_type => Err(PerformError::from(format!("Unknown job type {job_type}")))?,
                    })
                }

            }
        }
    }
}

macro_rules! job_variant_to_value {
    () => {
        Ok(serde_json::Value::Null)
    };
    ($content:ident) => {
        serde_json::to_value($content)
    };
}

macro_rules! job_variant_from_value {
    ($variant:ident $value:ident) => {
        Self::$variant
    };
    ($variant:ident $value:ident $content:ident) => {
        Self::$variant(serde_json::from_value($value)?)
    };
}

jobs! {
    pub enum Job {
        DailyDbMaintenance,
        DumpDb(DumpDbJob),
        NormalizeIndex(NormalizeIndexJob),
        RenderAndUploadReadme(RenderAndUploadReadmeJob),
        SquashIndex,
        SyncToGitIndex(SyncToIndexJob),
        SyncToSparseIndex(SyncToIndexJob),
        UpdateDownloads,
    }
}

/// Database state that is passed to `Job::perform()`.
pub(crate) struct PerformState<'a> {
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
    fn fresh_connection(
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

        let to_git = Self::sync_to_git_index(krate.to_string());
        let to_git = deduplicated_select_query(to_git.as_type_str(), to_git.to_value()?);

        let to_sparse = Self::sync_to_sparse_index(krate.to_string());
        let to_sparse = deduplicated_select_query(to_sparse.as_type_str(), to_sparse.to_value()?);

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

    pub fn daily_db_maintenance() -> Self {
        Self::DailyDbMaintenance
    }

    pub fn dump_db(database_url: String, target_name: String) -> Self {
        Self::DumpDb(DumpDbJob {
            database_url,
            target_name,
        })
    }

    pub fn normalize_index(dry_run: bool) -> Self {
        Self::NormalizeIndex(NormalizeIndexJob { dry_run })
    }

    pub fn render_and_upload_readme(
        version_id: i32,
        text: String,
        readme_path: String,
        base_url: Option<String>,
        pkg_path_in_vcs: Option<String>,
    ) -> Self {
        Self::RenderAndUploadReadme(RenderAndUploadReadmeJob {
            version_id,
            text,
            readme_path,
            base_url,
            pkg_path_in_vcs,
        })
    }

    pub fn squash_index() -> Self {
        Self::SquashIndex
    }

    pub fn sync_to_git_index<T: ToString>(krate: T) -> Self {
        Self::SyncToGitIndex(SyncToIndexJob {
            krate: krate.to_string(),
        })
    }

    pub fn sync_to_sparse_index<T: ToString>(krate: T) -> Self {
        Self::SyncToSparseIndex(SyncToIndexJob {
            krate: krate.to_string(),
        })
    }

    pub fn update_downloads() -> Self {
        Self::UpdateDownloads
    }

    pub fn enqueue(&self, conn: &mut PgConnection) -> Result<(), EnqueueError> {
        self.enqueue_with_priority(conn, PRIORITY_DEFAULT)
    }

    #[instrument(name = "swirl.enqueue", skip(self, conn), fields(message = self.as_type_str()))]
    pub fn enqueue_with_priority(
        &self,
        conn: &mut PgConnection,
        job_priority: i16,
    ) -> Result<(), EnqueueError> {
        let job_data = self.to_value()?;
        diesel::insert_into(background_jobs::table)
            .values((
                background_jobs::job_type.eq(self.as_type_str()),
                background_jobs::data.eq(job_data),
                background_jobs::priority.eq(job_priority),
            ))
            .execute(conn)?;
        Ok(())
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
            Job::DailyDbMaintenance => {
                worker::perform_daily_db_maintenance(&mut *state.fresh_connection()?)
            }
            Job::DumpDb(job) => worker::perform_dump_db(job, env),
            Job::SquashIndex => worker::perform_index_squash(env),
            Job::NormalizeIndex(args) => worker::perform_normalize_index(env, args),
            Job::RenderAndUploadReadme(job) => {
                worker::perform_render_and_upload_readme(job, state.conn, env)
            }
            Job::SyncToGitIndex(args) => worker::sync_to_git_index(env, state.conn, &args.krate),
            Job::SyncToSparseIndex(args) => {
                worker::sync_to_sparse_index(env, state.conn, &args.krate)
            }
            Job::UpdateDownloads => {
                worker::perform_update_downloads(&mut *state.fresh_connection()?)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DumpDbJob {
    pub(super) database_url: String,
    pub(super) target_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SyncToIndexJob {
    pub(super) krate: String,
}

#[derive(Serialize, Deserialize)]
pub struct NormalizeIndexJob {
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RenderAndUploadReadmeJob {
    pub(super) version_id: i32,
    pub(super) text: String,
    pub(super) readme_path: String,
    pub(super) base_url: Option<String>,
    pub(super) pkg_path_in_vcs: Option<String>,
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
