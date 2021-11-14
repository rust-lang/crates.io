//! The `worker` module contains all the tasks that can be queued up for the
//! background worker process to work on. This includes recurring tasks like
//! the daily database maintenance, but also operations like rendering READMEs
//! and uploading them to S3.

mod daily_db_maintenance;
pub mod dump_db;
mod git;
mod readmes;
mod update_downloads;

pub use daily_db_maintenance::daily_db_maintenance;
pub use dump_db::dump_db;
pub use git::{add_crate, squash_index, sync_yanked};
pub use readmes::render_and_upload_readme;
pub use update_downloads::update_downloads;
