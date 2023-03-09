//! The `worker` module contains all the tasks that can be queued up for the
//! background worker process to work on. This includes recurring tasks like
//! the daily database maintenance, but also operations like rendering READMEs
//! and uploading them to S3.

pub mod cloudfront;
mod daily_db_maintenance;
pub mod dump_db;
mod git;
mod readmes;
mod update_downloads;

pub use daily_db_maintenance::daily_db_maintenance;
pub use dump_db::dump_db;
pub use git::{add_crate, fix_features2, normalize_index, squash_index, sync_yanked};
pub use readmes::render_and_upload_readme;
pub use update_downloads::update_downloads;

pub(crate) use daily_db_maintenance::perform_daily_db_maintenance;
pub(crate) use dump_db::perform_dump_db;
pub(crate) use git::{
    perform_fix_features2, perform_index_add_crate, perform_index_squash,
    perform_index_sync_to_http, perform_index_update_yanked, perform_normalize_index,
};
pub(crate) use readmes::perform_render_and_upload_readme;
pub(crate) use update_downloads::perform_update_downloads;
