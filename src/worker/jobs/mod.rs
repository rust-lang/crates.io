use crate::util::diesel::Conn;
use crates_io_worker::{BackgroundJob, EnqueueError};
use std::fmt::Display;

mod archive_version_downloads;
mod daily_db_maintenance;
mod delete_crate;
mod downloads;
pub mod dump_db;
mod expiry_notification;
mod index;
mod index_version_downloads_archive;
mod readmes;
pub mod rss;
mod send_publish_notifications;
mod sync_admins;
mod typosquat;
mod update_default_version;

pub use self::archive_version_downloads::ArchiveVersionDownloads;
pub use self::daily_db_maintenance::DailyDbMaintenance;
pub use self::delete_crate::DeleteCrateFromStorage;
pub use self::downloads::{
    CleanProcessedLogFiles, ProcessCdnLog, ProcessCdnLogQueue, UpdateDownloads,
};
pub use self::dump_db::DumpDb;
pub use self::expiry_notification::SendTokenExpiryNotifications;
pub use self::index::{NormalizeIndex, SquashIndex, SyncToGitIndex, SyncToSparseIndex};
pub use self::index_version_downloads_archive::IndexVersionDownloadsArchive;
pub use self::readmes::RenderAndUploadReadme;
pub use self::send_publish_notifications::SendPublishNotificationsJob;
pub use self::sync_admins::SyncAdmins;
pub use self::typosquat::CheckTyposquat;
pub use self::update_default_version::UpdateDefaultVersion;

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
    conn: &mut impl Conn,
) -> Result<(), EnqueueError> {
    let krate = krate.to_string();

    SyncToGitIndex::new(krate.clone()).enqueue(conn)?;
    SyncToSparseIndex::new(krate).enqueue(conn)?;

    Ok(())
}
