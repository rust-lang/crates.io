use crate::dialoguer;
use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use clap::builder::ArgAction;
use crates_io::db;
use crates_io::schema::crates;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;
use crates_io_worker::schema::background_jobs;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use tracing::warn;

const BATCH_SIZE: usize = 1000;

async fn enqueue_jobs<T: BackgroundJob>(
    conn: &mut AsyncPgConnection,
    jobs: &[T],
    priority: i16,
) -> Result<()> {
    for chunk in jobs.chunks(BATCH_SIZE) {
        let values = chunk
            .iter()
            .map(|job| {
                Ok((
                    background_jobs::job_type.eq(T::JOB_NAME),
                    background_jobs::data.eq(serde_json::to_value(job)?),
                    background_jobs::priority.eq(priority),
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        diesel::insert_into(background_jobs::table)
            .values(&values)
            .execute(conn)
            .await?;
    }

    Ok(())
}

#[derive(clap::Parser, Debug)]
#[command(
    name = "sync-index",
    about = "Synchronize crate index data to git and sparse indexes"
)]
pub struct Opts {
    /// Names of the crates to synchronize
    #[arg(required_unless_present = "updated_before")]
    names: Vec<String>,

    /// Skip syncing to the git index
    #[arg(long = "no-git", action = ArgAction::SetFalse)]
    git: bool,

    /// Skip syncing to the sparse index
    #[arg(long = "no-sparse", action = ArgAction::SetFalse)]
    sparse: bool,

    /// Number of crates per bulk sync job (enables batch mode)
    #[arg(long, requires = "commit_message")]
    batch_size: Option<usize>,

    /// Commit message for bulk sync jobs
    #[arg(long, requires = "batch_size")]
    commit_message: Option<String>,

    /// Sync all crates with `updated_at` before this date (format: YYYY-MM-DD)
    #[arg(
        long,
        value_name = "DATE",
        requires = "batch_size",
        conflicts_with = "names"
    )]
    updated_before: Option<NaiveDate>,

    /// Priority for the enqueued jobs
    #[arg(long)]
    priority: Option<i16>,
}

pub async fn run(opts: Opts) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;

    let crate_names = resolve_crate_names(&mut conn, &opts).await?;

    let num_crates = crate_names.len();

    if num_crates == 0 {
        println!("No crates to sync");
        return Ok(());
    }

    if !confirm_sync(&opts, num_crates).await? {
        return Ok(());
    }

    enqueue_sync_jobs(&mut conn, &crate_names, &opts).await
}

/// Determines which crate names to sync based on the provided options.
///
/// When `updated_before` is set, queries for all crates updated before
/// that date. Otherwise, uses the explicitly provided names, warning
/// about any that don't exist in the database.
async fn resolve_crate_names(conn: &mut AsyncPgConnection, opts: &Opts) -> Result<Vec<String>> {
    if let Some(date) = opts.updated_before {
        let datetime = Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN));

        let names = crates::table
            .filter(crates::updated_at.lt(datetime))
            .select(crates::name)
            .order(crates::name)
            .load(conn)
            .await?;

        return Ok(names);
    }

    // Check which crates exist in the database. Crates that don't
    // exist will still be synced, which removes them from the index.
    let existing_crates: Vec<String> = crates::table
        .filter(crates::name.eq_any(&opts.names))
        .select(crates::name)
        .load(conn)
        .await?;

    for name in &opts.names {
        if !existing_crates.contains(name) {
            warn!(
                "Crate `{name}` does not exist in the database and will be removed from the index."
            );
        }
    }

    Ok(opts.names.clone())
}

/// Shows a confirmation prompt when batch mode is used.
///
/// Returns `true` if the sync should proceed, `false` if the user
/// declined or no confirmation was needed but nothing to do.
async fn confirm_sync(opts: &Opts, num_crates: usize) -> Result<bool> {
    let Some(batch_size) = opts.batch_size else {
        return Ok(true);
    };

    let mut prompt_parts = Vec::new();

    if opts.git {
        let num_batches = num_crates.div_ceil(batch_size);
        prompt_parts.push(format!(
            "This will sync {num_crates} crate{} to the git index in {num_batches} batch{}.",
            if num_crates == 1 { "" } else { "s" },
            if num_batches == 1 { "" } else { "es" }
        ));
    }

    if opts.sparse {
        prompt_parts.push(format!(
            "This will enqueue {num_crates} sparse index sync job{}.",
            if num_crates == 1 { "" } else { "s" }
        ));
    }

    if prompt_parts.is_empty() {
        return Ok(true);
    }

    println!("{}", prompt_parts.join("\n"));
    dialoguer::confirm("Do you want to continue?").await
}

/// Enqueues git and/or sparse index sync jobs for the given crate names.
async fn enqueue_sync_jobs(
    conn: &mut AsyncPgConnection,
    crate_names: &[String],
    opts: &Opts,
) -> Result<()> {
    conn.transaction(async |conn| {
        // Handle git index sync
        if opts.git {
            if let Some(batch_size) = opts.batch_size
                && let Some(commit_message) = opts.commit_message.as_ref()
            {
                let priority = opts.priority.unwrap_or(jobs::BulkSyncToGitIndex::PRIORITY);

                let jobs: Vec<_> = crate_names
                    .chunks(batch_size)
                    .map(|chunk| jobs::BulkSyncToGitIndex::new(chunk.to_vec(), commit_message))
                    .collect();

                println!("Enqueueing {} BulkSyncToGitIndex jobs", jobs.len());
                enqueue_jobs(conn, &jobs, priority).await?;
            } else {
                let priority = opts.priority.unwrap_or(jobs::SyncToGitIndex::PRIORITY);
                let jobs: Vec<_> = crate_names.iter().map(jobs::SyncToGitIndex::new).collect();

                println!("Enqueueing {} SyncToGitIndex jobs", jobs.len());
                enqueue_jobs(conn, &jobs, priority).await?;
            }
        }

        // Handle sparse index sync (always per-crate)
        if opts.sparse {
            let priority = opts.priority.unwrap_or(jobs::SyncToSparseIndex::PRIORITY);
            let jobs: Vec<_> = crate_names
                .iter()
                .map(jobs::SyncToSparseIndex::new)
                .collect();

            println!("Enqueueing {} SyncToSparseIndex jobs", jobs.len());
            enqueue_jobs(conn, &jobs, priority).await?;
        }

        Ok(())
    })
    .await
}
