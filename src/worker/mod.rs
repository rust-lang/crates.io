//! The `worker` module contains all the tasks that can be queued up for the
//! background worker process to work on. This includes recurring tasks like
//! the daily database maintenance, but also operations like rendering READMEs
//! and uploading them to S3.

pub mod cloudfront;
mod daily_db_maintenance;
pub mod dump_db;
pub mod fastly;
mod git;
mod readmes;
mod update_downloads;

pub(crate) use daily_db_maintenance::DailyDbMaintenanceJob;
pub(crate) use dump_db::DumpDbJob;
pub(crate) use git::{NormalizeIndexJob, SquashIndexJob, SyncToGitIndexJob, SyncToSparseIndexJob};
pub(crate) use readmes::RenderAndUploadReadmeJob;
pub(crate) use update_downloads::UpdateDownloadsJob;

use crate::swirl::Runner;

pub trait RunnerExt {
    fn register_crates_io_job_types(self) -> Self;
}

impl RunnerExt for Runner {
    fn register_crates_io_job_types(self) -> Self {
        self.register_job_type::<DailyDbMaintenanceJob>()
            .register_job_type::<DumpDbJob>()
            .register_job_type::<NormalizeIndexJob>()
            .register_job_type::<RenderAndUploadReadmeJob>()
            .register_job_type::<SquashIndexJob>()
            .register_job_type::<SyncToGitIndexJob>()
            .register_job_type::<SyncToSparseIndexJob>()
            .register_job_type::<UpdateDownloadsJob>()
    }
}
