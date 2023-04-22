use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use reqwest::blocking::Client;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::db::ConnectionPool;
use crate::swirl::errors::EnqueueError;
use crate::swirl::PerformError;
use crate::uploaders::Uploader;
use crate::worker;
use crate::worker::cloudfront::CloudFront;
use cargo_registry_index::Repository;

pub enum Job {
    DailyDbMaintenance,
    DumpDb(DumpDbJob),
    IndexAddCrate(IndexAddCrateJob),
    IndexSquash,
    IndexSyncToHttp(IndexSyncToHttpJob),
    SyncToGitIndex(SyncToIndexJob),
    SyncToSparseIndex(SyncToIndexJob),
    IndexUpdateYanked(IndexUpdateYankedJob),
    NormalizeIndex(NormalizeIndexJob),
    RenderAndUploadReadme(RenderAndUploadReadmeJob),
    UpdateDownloads,
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

impl Job {
    const DAILY_DB_MAINTENANCE: &str = "daily_db_maintenance";
    const DUMP_DB: &str = "dump_db";
    const INDEX_ADD_CRATE: &str = "add_crate";
    const INDEX_SQUASH: &str = "squash_index";
    const INDEX_SYNC_TO_HTTP: &str = "update_crate_index";
    const INDEX_UPDATE_YANKED: &str = "sync_yanked";
    const NORMALIZE_INDEX: &str = "normalize_index";
    const RENDER_AND_UPLOAD_README: &str = "render_and_upload_readme";
    const SYNC_TO_GIT_INDEX: &str = "sync_to_git_index";
    const SYNC_TO_SPARSE_INDEX: &str = "sync_to_sparse_index";
    const UPDATE_DOWNLOADS: &str = "update_downloads";

    pub fn enqueue_sync_to_index<T: ToString>(
        krate: T,
        conn: &mut PgConnection,
    ) -> Result<(), EnqueueError> {
        use crate::schema::background_jobs::dsl::*;

        let to_git = Self::sync_to_git_index(krate.to_string());
        let to_git = (
            job_type.eq(to_git.as_type_str()),
            data.eq(to_git.to_value()?),
        );

        let to_sparse = Self::sync_to_sparse_index(krate.to_string());
        let to_sparse = (
            job_type.eq(to_sparse.as_type_str()),
            data.eq(to_sparse.to_value()?),
        );

        diesel::insert_into(background_jobs)
            .values(vec![to_git, to_sparse])
            .execute(conn)?;

        Ok(())
    }

    pub fn add_crate(krate: cargo_registry_index::Crate) -> Self {
        Self::IndexAddCrate(IndexAddCrateJob { krate })
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
        Self::IndexSquash
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

    pub fn sync_yanked(krate: String, version_num: String) -> Self {
        Self::IndexUpdateYanked(IndexUpdateYankedJob { krate, version_num })
    }

    pub fn update_crate_index(crate_name: String) -> Self {
        Self::IndexSyncToHttp(IndexSyncToHttpJob { crate_name })
    }

    pub fn update_downloads() -> Self {
        Self::UpdateDownloads
    }

    fn as_type_str(&self) -> &'static str {
        match self {
            Job::DailyDbMaintenance => Self::DAILY_DB_MAINTENANCE,
            Job::DumpDb(_) => Self::DUMP_DB,
            Job::IndexAddCrate(_) => Self::INDEX_ADD_CRATE,
            Job::IndexSquash => Self::INDEX_SQUASH,
            Job::IndexSyncToHttp(_) => Self::INDEX_SYNC_TO_HTTP,
            Job::IndexUpdateYanked(_) => Self::INDEX_UPDATE_YANKED,
            Job::NormalizeIndex(_) => Self::NORMALIZE_INDEX,
            Job::RenderAndUploadReadme(_) => Self::RENDER_AND_UPLOAD_README,
            Job::SyncToGitIndex(_) => Self::SYNC_TO_GIT_INDEX,
            Job::SyncToSparseIndex(_) => Self::SYNC_TO_SPARSE_INDEX,
            Job::UpdateDownloads => Self::UPDATE_DOWNLOADS,
        }
    }

    fn to_value(&self) -> serde_json::Result<serde_json::Value> {
        match self {
            Job::DailyDbMaintenance => Ok(serde_json::Value::Null),
            Job::DumpDb(inner) => serde_json::to_value(inner),
            Job::IndexAddCrate(inner) => serde_json::to_value(inner),
            Job::IndexSquash => Ok(serde_json::Value::Null),
            Job::IndexSyncToHttp(inner) => serde_json::to_value(inner),
            Job::IndexUpdateYanked(inner) => serde_json::to_value(inner),
            Job::NormalizeIndex(inner) => serde_json::to_value(inner),
            Job::RenderAndUploadReadme(inner) => serde_json::to_value(inner),
            Job::SyncToGitIndex(inner) => serde_json::to_value(inner),
            Job::SyncToSparseIndex(inner) => serde_json::to_value(inner),
            Job::UpdateDownloads => Ok(serde_json::Value::Null),
        }
    }

    pub fn enqueue(&self, conn: &mut PgConnection) -> Result<(), EnqueueError> {
        use crate::schema::background_jobs::dsl::*;

        let job_data = self.to_value()?;
        diesel::insert_into(background_jobs)
            .values((job_type.eq(self.as_type_str()), data.eq(job_data)))
            .execute(conn)?;
        Ok(())
    }

    pub(super) fn from_value(
        job_type: &str,
        value: serde_json::Value,
    ) -> Result<Self, PerformError> {
        use serde_json::from_value;
        Ok(match job_type {
            Self::DAILY_DB_MAINTENANCE => Job::DailyDbMaintenance,
            Self::DUMP_DB => Job::DumpDb(from_value(value)?),
            Self::INDEX_ADD_CRATE => Job::IndexAddCrate(from_value(value)?),
            Self::INDEX_SQUASH => Job::IndexSquash,
            Self::INDEX_SYNC_TO_HTTP => Job::IndexSyncToHttp(from_value(value)?),
            Self::INDEX_UPDATE_YANKED => Job::IndexUpdateYanked(from_value(value)?),
            Self::NORMALIZE_INDEX => Job::NormalizeIndex(from_value(value)?),
            Self::RENDER_AND_UPLOAD_README => Job::RenderAndUploadReadme(from_value(value)?),
            Self::SYNC_TO_GIT_INDEX => Job::SyncToGitIndex(from_value(value)?),
            Self::SYNC_TO_SPARSE_INDEX => Job::SyncToSparseIndex(from_value(value)?),
            Self::UPDATE_DOWNLOADS => Job::UpdateDownloads,
            job_type => Err(PerformError::from(format!("Unknown job type {job_type}")))?,
        })
    }

    pub(super) fn perform(
        self,
        env: &Option<Environment>,
        state: PerformState<'_>,
    ) -> Result<(), PerformError> {
        let PerformState { conn, pool } = state;
        let env = env
            .as_ref()
            .expect("Application should configure a background runner environment");
        match self {
            Job::DailyDbMaintenance => {
                worker::perform_daily_db_maintenance(&mut *fresh_connection(pool)?)
            }
            Job::DumpDb(args) => worker::perform_dump_db(env, args.database_url, args.target_name),
            Job::IndexAddCrate(args) => worker::perform_index_add_crate(env, conn, &args.krate),
            Job::IndexSquash => worker::perform_index_squash(env),
            Job::IndexSyncToHttp(args) => worker::perform_index_sync_to_http(env, args.crate_name),
            Job::IndexUpdateYanked(args) => {
                worker::perform_index_update_yanked(env, conn, &args.krate, &args.version_num)
            }
            Job::NormalizeIndex(args) => worker::perform_normalize_index(env, args),
            Job::RenderAndUploadReadme(args) => worker::perform_render_and_upload_readme(
                conn,
                env,
                args.version_id,
                &args.text,
                &args.readme_path,
                args.base_url.as_deref(),
                args.pkg_path_in_vcs.as_deref(),
            ),
            Job::SyncToGitIndex(args) => worker::sync_to_git_index(env, conn, &args.krate),
            Job::SyncToSparseIndex(args) => worker::sync_to_sparse_index(env, conn, &args.krate),
            Job::UpdateDownloads => worker::perform_update_downloads(&mut *fresh_connection(pool)?),
        }
    }
}

/// A helper function for jobs needing a fresh connection (i.e. not already within a transaction).
///
/// This will error when run from our main test framework, as there most work is expected to be
/// done within an existing transaction.
fn fresh_connection(
    pool: Option<ConnectionPool>,
) -> Result<PooledConnection<ConnectionManager<PgConnection>>, PerformError> {
    let Some(pool) = pool else {
        // In production a pool should be available. This can only be hit in tests, which don't
        // provide the pool.
        return Err(String::from("Database pool was unavailable").into());
    };
    Ok(pool.get()?)
}

#[derive(Serialize, Deserialize)]
pub struct DumpDbJob {
    pub(super) database_url: String,
    pub(super) target_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct IndexAddCrateJob {
    pub(super) krate: cargo_registry_index::Crate,
}

#[derive(Serialize, Deserialize)]
pub struct IndexSyncToHttpJob {
    pub(super) crate_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SyncToIndexJob {
    pub(super) krate: String,
}

#[derive(Serialize, Deserialize)]
pub struct IndexUpdateYankedJob {
    pub(super) krate: String,
    pub(super) version_num: String,
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
    index: Arc<Mutex<Repository>>,
    pub uploader: Uploader,
    http_client: AssertUnwindSafe<Client>,
    cloudfront: Option<CloudFront>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            index: self.index.clone(),
            uploader: self.uploader.clone(),
            http_client: AssertUnwindSafe(self.http_client.0.clone()),
            cloudfront: self.cloudfront.clone(),
        }
    }
}

impl Environment {
    pub fn new(
        index: Repository,
        uploader: Uploader,
        http_client: Client,
        cloudfront: Option<CloudFront>,
    ) -> Self {
        Self::new_shared(
            Arc::new(Mutex::new(index)),
            uploader,
            http_client,
            cloudfront,
        )
    }

    pub fn new_shared(
        index: Arc<Mutex<Repository>>,
        uploader: Uploader,
        http_client: Client,
        cloudfront: Option<CloudFront>,
    ) -> Self {
        Self {
            index,
            uploader,
            http_client: AssertUnwindSafe(http_client),
            cloudfront,
        }
    }

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
}
