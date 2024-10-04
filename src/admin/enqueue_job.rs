use crate::db;
use crate::schema::{background_jobs, crates};
use crate::worker::jobs;
use anyhow::Result;
use chrono::NaiveDate;
use crates_io_worker::BackgroundJob;
use diesel::dsl::exists;
use diesel::prelude::*;

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

pub fn run(command: Command) -> Result<()> {
    let conn = &mut db::oneoff_connection()?;
    println!("Enqueueing background job: {command:?}");

    match command {
        Command::ArchiveVersionDownloads { before } => {
            before
                .map(jobs::ArchiveVersionDownloads::before)
                .unwrap_or_default()
                .enqueue(conn)?;
        }
        Command::IndexVersionDownloadsArchive => {
            jobs::IndexVersionDownloadsArchive.enqueue(conn)?;
        }
        Command::UpdateDownloads => {
            let count: i64 = background_jobs::table
                .filter(background_jobs::job_type.eq(jobs::UpdateDownloads::JOB_NAME))
                .count()
                .get_result(conn)?;

            if count > 0 {
                println!(
                    "Did not enqueue {}, existing job already in progress",
                    jobs::UpdateDownloads::JOB_NAME
                );
            } else {
                jobs::UpdateDownloads.enqueue(conn)?;
            }
        }
        Command::CleanProcessedLogFiles => {
            jobs::CleanProcessedLogFiles.enqueue(conn)?;
        }
        Command::DumpDb => {
            jobs::DumpDb.enqueue(conn)?;
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

                if diesel::select(exists(query)).get_result(conn)? {
                    info!(
                        "Did not enqueue {}, existing job already in progress",
                        jobs::SyncAdmins::JOB_NAME
                    );
                    return Ok(());
                }
            }

            jobs::SyncAdmins.enqueue(conn)?;
        }
        Command::DailyDbMaintenance => {
            jobs::DailyDbMaintenance.enqueue(conn)?;
        }
        Command::ProcessCdnLogQueue(job) => {
            job.enqueue(conn)?;
        }
        Command::SquashIndex => {
            jobs::SquashIndex.enqueue(conn)?;
        }
        Command::NormalizeIndex { dry_run } => {
            jobs::NormalizeIndex::new(dry_run).enqueue(conn)?;
        }
        Command::CheckTyposquat { name } => {
            // The job will fail if the crate doesn't actually exist, so let's check that up front.
            if crates::table
                .filter(crates::name.eq(&name))
                .count()
                .get_result::<i64>(conn)?
                == 0
            {
                anyhow::bail!(
                    "cannot enqueue a typosquat check for a crate that doesn't exist: {name}"
                );
            }

            jobs::CheckTyposquat::new(&name).enqueue(conn)?;
        }
        Command::SendTokenExpiryNotifications => {
            jobs::SendTokenExpiryNotifications.enqueue(conn)?;
        }
        Command::SyncCratesFeed => {
            jobs::rss::SyncCratesFeed.enqueue(conn)?;
        }
        Command::SyncToGitIndex { name } => {
            jobs::SyncToGitIndex::new(name).enqueue(conn)?;
        }
        Command::SyncToSparseIndex { name } => {
            jobs::SyncToSparseIndex::new(name).enqueue(conn)?;
        }
        Command::SyncUpdatesFeed => {
            jobs::rss::SyncUpdatesFeed.enqueue(conn)?;
        }
    };

    Ok(())
}
