use crate::db;
use crate::schema::{background_jobs, crates};
use crate::worker::jobs;
use anyhow::Result;
use chrono::NaiveDate;
use crates_io_worker::BackgroundJob;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(clap::Parser, Debug)]
#[command(
    name = "enqueue-job",
    about = "Add a job to the background worker queue",
    rename_all = "snake_case"
)]
pub enum Command {
    ArchiveVersionDownloads {
        #[arg(long)]
        /// The date before which to archive version downloads (default: 90 days ago)
        before: Option<NaiveDate>,
    },
    IndexVersionDownloadsArchive,
    UpdateDownloads,
    CleanProcessedLogFiles,
    DumpDb,
    DailyDbMaintenance,
    SquashIndex,
    NormalizeIndex {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    CheckTyposquat {
        #[arg()]
        name: String,
    },
    ProcessCdnLogQueue(jobs::ProcessCdnLogQueue),
    SyncAdmins {
        /// Force a sync even if one is already in progress
        #[arg(long)]
        force: bool,
    },
    SendTokenExpiryNotifications,
    SyncCratesFeed,
    SyncToGitIndex {
        name: String,
    },
    SyncToSparseIndex {
        name: String,
    },
    SyncUpdatesFeed,
}

pub async fn run(command: Command) -> Result<()> {
    let mut conn = db::oneoff_async_connection().await?;
    println!("Enqueueing background job: {command:?}");

    match command {
        Command::ArchiveVersionDownloads { before } => {
            before
                .map(jobs::ArchiveVersionDownloads::before)
                .unwrap_or_default()
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::IndexVersionDownloadsArchive => {
            jobs::IndexVersionDownloadsArchive
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::UpdateDownloads => {
            let count: i64 = background_jobs::table
                .filter(background_jobs::job_type.eq(jobs::UpdateDownloads::JOB_NAME))
                .count()
                .get_result(&mut conn)
                .await?;

            if count > 0 {
                println!(
                    "Did not enqueue {}, existing job already in progress",
                    jobs::UpdateDownloads::JOB_NAME
                );
            } else {
                jobs::UpdateDownloads.async_enqueue(&mut conn).await?;
            }
        }
        Command::CleanProcessedLogFiles => {
            jobs::CleanProcessedLogFiles
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::DumpDb => {
            jobs::DumpDb.async_enqueue(&mut conn).await?;
        }
        Command::SyncAdmins { force } => {
            if !force {
                // By default, we don't want to enqueue a sync if one is already
                // in progress. If a sync fails due to e.g. an expired pinned
                // certificate we don't want to keep adding new jobs to the
                // queue, since the existing job will be retried until it
                // succeeds.

                let query = background_jobs::table
                    .filter(background_jobs::job_type.eq(jobs::SyncAdmins::JOB_NAME));

                if diesel::select(exists(query)).get_result(&mut conn).await? {
                    info!(
                        "Did not enqueue {}, existing job already in progress",
                        jobs::SyncAdmins::JOB_NAME
                    );
                    return Ok(());
                }
            }

            jobs::SyncAdmins.async_enqueue(&mut conn).await?;
        }
        Command::DailyDbMaintenance => {
            jobs::DailyDbMaintenance.async_enqueue(&mut conn).await?;
        }
        Command::ProcessCdnLogQueue(job) => {
            job.async_enqueue(&mut conn).await?;
        }
        Command::SquashIndex => {
            jobs::SquashIndex.async_enqueue(&mut conn).await?;
        }
        Command::NormalizeIndex { dry_run } => {
            jobs::NormalizeIndex::new(dry_run)
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::CheckTyposquat { name } => {
            // The job will fail if the crate doesn't actually exist, so let's check that up front.
            if crates::table
                .filter(crates::name.eq(&name))
                .count()
                .get_result::<i64>(&mut conn)
                .await?
                == 0
            {
                anyhow::bail!(
                    "cannot enqueue a typosquat check for a crate that doesn't exist: {name}"
                );
            }

            jobs::CheckTyposquat::new(&name)
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::SendTokenExpiryNotifications => {
            jobs::SendTokenExpiryNotifications
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::SyncCratesFeed => {
            jobs::rss::SyncCratesFeed.async_enqueue(&mut conn).await?;
        }
        Command::SyncToGitIndex { name } => {
            jobs::SyncToGitIndex::new(name)
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::SyncToSparseIndex { name } => {
            jobs::SyncToSparseIndex::new(name)
                .async_enqueue(&mut conn)
                .await?;
        }
        Command::SyncUpdatesFeed => {
            jobs::rss::SyncUpdatesFeed.async_enqueue(&mut conn).await?;
        }
    };

    Ok(())
}
