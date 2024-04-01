use crates_io_worker::schema::background_jobs;
use crates_io_worker::{BackgroundJob, EnqueueError};
use diesel::dsl::{exists, not};
use diesel::prelude::*;
use diesel::sql_types::{Int2, Jsonb, Text};
use std::fmt::Display;

mod archive_version_downloads;
mod daily_db_maintenance;
mod downloads;
pub mod dump_db;
mod expiry_notification;
mod git;
mod readmes;
mod sync_admins;
mod typosquat;
mod update_default_version;

pub use self::archive_version_downloads::ArchiveVersionDownloads;
pub use self::daily_db_maintenance::DailyDbMaintenance;
pub use self::downloads::{
    CleanProcessedLogFiles, ProcessCdnLog, ProcessCdnLogQueue, UpdateDownloads,
};
pub use self::dump_db::DumpDb;
pub use self::expiry_notification::CheckAboutToExpireToken;
pub use self::git::{NormalizeIndex, SquashIndex, SyncToGitIndex, SyncToSparseIndex};
pub use self::readmes::RenderAndUploadReadme;
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
        SyncToGitIndex::JOB_NAME,
        serde_json::to_value(SyncToGitIndex::new(krate.to_string()))?,
        SyncToGitIndex::PRIORITY,
    );

    let to_sparse = deduplicated_select_query(
        SyncToSparseIndex::JOB_NAME,
        serde_json::to_value(SyncToSparseIndex::new(krate.to_string()))?,
        SyncToSparseIndex::PRIORITY,
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
