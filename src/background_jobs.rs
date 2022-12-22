use diesel::prelude::*;
use reqwest::blocking::Client;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::db::DieselPool;
use crate::swirl::errors::EnqueueError;
use crate::swirl::PerformError;
use crate::uploaders::Uploader;
use crate::worker;
use crate::worker::cloudfront::CloudFront;
use cargo_registry_index::Repository;

pub enum Job {
    DailyDbMaintenance(DailyDbMaintenanceJob),
    DumpDb(DumpDbJob),
    IndexAddCrate(IndexAddCrateJob),
    IndexSquash(IndexSquashJob),
    IndexSyncToHttp(IndexSyncToHttpJob),
    IndexUpdateYanked(IndexUpdateYankedJob),
    RenderAndUploadReadme(RenderAndUploadReadmeJob),
    UpdateDownloads(UpdateDownloadsJob),
}

impl Job {
    const DAILY_DB_MAINTENANCE: &str = "daily_db_maintenance";
    const DUMP_DB: &str = "dump_db";
    const INDEX_ADD_CRATE: &str = "add_crate";
    const INDEX_SQUASH: &str = "squash_index";
    const INDEX_SYNC_TO_HTTP: &str = "update_crate_index";
    const INDEX_UPDATE_YANKED: &str = "sync_yanked";
    const RENDER_AND_UPLOAD_README: &str = "render_and_upload_readme";
    const UPDATE_DOWNLOADS: &str = "update_downloads";

    fn as_type_str(&self) -> &'static str {
        match self {
            Job::DailyDbMaintenance(_) => Self::DAILY_DB_MAINTENANCE,
            Job::DumpDb(_) => Self::DUMP_DB,
            Job::IndexAddCrate(_) => Self::INDEX_ADD_CRATE,
            Job::IndexSquash(_) => Self::INDEX_SQUASH,
            Job::IndexSyncToHttp(_) => Self::INDEX_SYNC_TO_HTTP,
            Job::IndexUpdateYanked(_) => Self::INDEX_UPDATE_YANKED,
            Job::RenderAndUploadReadme(_) => Self::RENDER_AND_UPLOAD_README,
            Job::UpdateDownloads(_) => Self::UPDATE_DOWNLOADS,
        }
    }

    fn to_value(&self) -> serde_json::Result<serde_json::Value> {
        match self {
            Job::DailyDbMaintenance(inner) => serde_json::to_value(inner),
            Job::DumpDb(inner) => serde_json::to_value(inner),
            Job::IndexAddCrate(inner) => serde_json::to_value(inner),
            Job::IndexSquash(inner) => serde_json::to_value(inner),
            Job::IndexSyncToHttp(inner) => serde_json::to_value(inner),
            Job::IndexUpdateYanked(inner) => serde_json::to_value(inner),
            Job::RenderAndUploadReadme(inner) => serde_json::to_value(inner),
            Job::UpdateDownloads(inner) => serde_json::to_value(inner),
        }
    }

    pub fn enqueue(&self, conn: &PgConnection) -> Result<(), EnqueueError> {
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
            Self::DAILY_DB_MAINTENANCE => Job::DailyDbMaintenance(from_value(value)?),
            Self::DUMP_DB => Job::DumpDb(from_value(value)?),
            Self::INDEX_ADD_CRATE => Job::IndexAddCrate(from_value(value)?),
            Self::INDEX_SQUASH => Job::IndexSquash(from_value(value)?),
            Self::INDEX_SYNC_TO_HTTP => Job::IndexSyncToHttp(from_value(value)?),
            Self::INDEX_UPDATE_YANKED => Job::IndexUpdateYanked(from_value(value)?),
            Self::RENDER_AND_UPLOAD_README => Job::RenderAndUploadReadme(from_value(value)?),
            Self::UPDATE_DOWNLOADS => Job::UpdateDownloads(from_value(value)?),
            job_type => Err(PerformError::from(format!("Unknown job type {job_type}")))?,
        })
    }

    pub(super) fn perform(
        self,
        env: &Option<Environment>,
        conn: &DieselPool,
    ) -> Result<(), PerformError> {
        let env = env
            .as_ref()
            .expect("Application should configure a background runner environment");
        match self {
            Job::DailyDbMaintenance(_) => {
                conn.with_connection(&worker::perform_daily_db_maintenance)
            }
            Job::DumpDb(args) => worker::perform_dump_db(env, args.database_url, args.target_name),
            Job::IndexAddCrate(args) => conn
                .with_connection(&|conn| worker::perform_index_add_crate(env, conn, &args.krate)),
            Job::IndexSquash(_) => worker::perform_index_squash(env),
            Job::IndexSyncToHttp(args) => worker::perform_index_sync_to_http(env, args.crate_name),
            Job::IndexUpdateYanked(args) => conn.with_connection(&|conn| {
                worker::perform_index_update_yanked(env, conn, &args.krate, &args.version_num)
            }),
            Job::RenderAndUploadReadme(args) => conn.with_connection(&|conn| {
                worker::perform_render_and_upload_readme(
                    conn,
                    env,
                    args.version_id,
                    &args.text,
                    &args.readme_path,
                    args.base_url.as_deref(),
                    args.pkg_path_in_vcs.as_deref(),
                )
            }),
            Job::UpdateDownloads(_) => conn.with_connection(&worker::perform_update_downloads),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DailyDbMaintenanceJob {}

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
pub struct IndexSquashJob {}

#[derive(Serialize, Deserialize)]
pub struct IndexSyncToHttpJob {
    pub(super) crate_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct IndexUpdateYankedJob {
    pub(super) krate: String,
    pub(super) version_num: String,
}

#[derive(Serialize, Deserialize)]
pub struct RenderAndUploadReadmeJob {
    pub(super) version_id: i32,
    pub(super) text: String,
    pub(super) readme_path: String,
    pub(super) base_url: Option<String>,
    pub(super) pkg_path_in_vcs: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateDownloadsJob {}

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
