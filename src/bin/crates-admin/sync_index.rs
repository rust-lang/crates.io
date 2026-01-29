use crate::dialoguer;
use anyhow::{Result, bail};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use clap::builder::ArgAction;
use crates_io::db;
use crates_io::schema::crates;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;
use crates_io_worker::schema::background_jobs;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

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

    /// Create a single git commit with this message instead of per-crate commits
    #[arg(long, value_name = "COMMIT_MESSAGE")]
    single_commit: Option<String>,

    /// Sync all crates with `updated_at` before this date (format: YYYY-MM-DD)
    #[arg(
        long,
        value_name = "DATE",
        requires = "single_commit",
        conflicts_with = "names"
    )]
    updated_before: Option<NaiveDate>,

    /// Priority for the enqueued jobs
    #[arg(long)]
    priority: Option<i16>,
}

pub async fn run(opts: Opts) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;

    // Determine which crates to sync
    let crate_names: Vec<String> = if let Some(date) = opts.updated_before {
        let datetime = Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN));

        crates::table
            .filter(crates::updated_at.lt(datetime))
            .select(crates::name)
            .order(crates::name)
            .load(&mut conn)
            .await?
    } else {
        // Validate all crates exist before enqueueing any jobs
        let existing_crates: Vec<String> = crates::table
            .filter(crates::name.eq_any(&opts.names))
            .select(crates::name)
            .load(&mut conn)
            .await?;

        let missing_crates: Vec<_> = opts
            .names
            .iter()
            .filter(|name| !existing_crates.contains(name))
            .collect();

        let num_missing_crates = missing_crates.len();
        if num_missing_crates == 1 {
            bail!("Crate {} does not exist", missing_crates[0]);
        } else if num_missing_crates > 1 {
            bail!("Crates {missing_crates:?} do not exist");
        }

        opts.names
    };

    let num_crates = crate_names.len();

    if num_crates == 0 {
        println!("No crates to sync");
        return Ok(());
    }

    // Show confirmation prompt when --single-commit is used
    if opts.single_commit.is_some() {
        let mut prompt_parts = Vec::new();

        if opts.git {
            prompt_parts.push(format!(
                "This will sync {num_crates} crate{} to the git index in a single commit.",
                if num_crates == 1 { "" } else { "s" }
            ));
        }

        if opts.sparse {
            prompt_parts.push(format!(
                "This will enqueue {num_crates} sparse index sync job{}.",
                if num_crates == 1 { "" } else { "s" }
            ));
        }

        if !prompt_parts.is_empty() {
            println!("{}", prompt_parts.join("\n"));
            if !dialoguer::confirm("Do you want to continue?").await? {
                return Ok(());
            }
        }
    }

    conn.transaction(|conn| {
        Box::pin(async move {
            // Handle git index sync
            if opts.git {
                if let Some(commit_message) = &opts.single_commit {
                    let priority = opts.priority.unwrap_or(jobs::BulkSyncToGitIndex::PRIORITY);
                    let job = jobs::BulkSyncToGitIndex::new(crate_names.clone(), commit_message);

                    println!("Enqueueing BulkSyncToGitIndex job for {num_crates} crates");
                    enqueue_jobs(conn, &[job], priority).await?;
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
    })
    .await
}
