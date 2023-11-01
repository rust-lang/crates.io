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
