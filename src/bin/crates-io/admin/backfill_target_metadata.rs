use anyhow::Context;
use crates_io::db;
use crates_io::schema::{background_jobs, default_versions, versions};
use crates_io::worker::jobs::BackfillTargetMetadata;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::num::NonZeroU32;

/// Options for the target metadata backfill command.
#[derive(clap::Parser, Debug)]
#[command(
    name = "backfill-target-metadata",
    about = "Queue target metadata backfill jobs for unfinished versions"
)]
pub struct Options {
    /// Number of versions to load per batch.
    #[arg(long, default_value = "1000")]
    batch_size: NonZeroU32,
}

/// Queues target metadata backfill jobs for all unfinished versions.
pub async fn run(options: Options) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to connect to the database")?;

    let enqueued = enqueue_jobs(&mut conn, options.batch_size).await?;
    println!("Enqueued {enqueued} target metadata backfill jobs");

    Ok(())
}

async fn enqueue_jobs(
    conn: &mut AsyncPgConnection,
    batch_size: NonZeroU32,
) -> anyhow::Result<usize> {
    let mut after_id = 0;
    let mut enqueued = 0;

    loop {
        let versions = VersionToBackfill::load(conn, after_id, batch_size).await?;
        let Some(last) = versions.last() else {
            break;
        };

        after_id = last.version_id;

        let jobs = versions
            .into_iter()
            .map(VersionToBackfill::into_job)
            .collect::<anyhow::Result<Vec<_>>>()?;

        enqueued += diesel::insert_into(background_jobs::table)
            .values(jobs)
            .execute(conn)
            .await?;
    }

    Ok(enqueued)
}

/// A crate version selected for target metadata backfilling.
#[derive(Debug, HasQuery)]
#[diesel(
    base_query = versions::table
        .left_join(default_versions::table.on(default_versions::version_id.eq(versions::id)))
)]
struct VersionToBackfill {
    #[diesel(select_expression = versions::id)]
    version_id: i32,
    #[diesel(select_expression = default_versions::crate_id.nullable().is_not_null())]
    is_default_version: bool,
}

impl VersionToBackfill {
    /// Loads the next page of versions without target metadata.
    async fn load(
        conn: &mut AsyncPgConnection,
        after_id: i32,
        batch_size: NonZeroU32,
    ) -> QueryResult<Vec<Self>> {
        Self::query()
            .filter(versions::target_metadata.is_null())
            .filter(versions::id.gt(after_id))
            .order(versions::id)
            .limit(i64::from(batch_size.get()))
            .load(conn)
            .await
    }

    /// Returns the backfill priority for this version.
    fn priority(&self) -> i16 {
        if self.is_default_version { -20 } else { -50 }
    }

    /// Converts this version into a serialized background job.
    fn into_job(self) -> anyhow::Result<NewBackgroundJob> {
        let job = BackfillTargetMetadata::new(self.version_id);
        let data = serde_json::to_value(job).context("Failed to serialize job data")?;

        Ok(NewBackgroundJob {
            job_type: BackfillTargetMetadata::JOB_NAME,
            data,
            priority: self.priority(),
        })
    }
}

/// A target metadata backfill job ready for bulk insertion.
#[derive(Debug, Insertable)]
#[diesel(table_name = background_jobs)]
struct NewBackgroundJob {
    job_type: &'static str,
    data: serde_json::Value,
    priority: i16,
}
