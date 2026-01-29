use anyhow::Result;
use chrono::NaiveDate;
use crates_io::db;
use crates_io::schema::{background_jobs, crates};
use crates_io::worker::jobs;
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
    CheckTyposquat {
        #[arg()]
        name: String,
    },
    CleanProcessedLogFiles,
    DailyDbMaintenance,
    DumpDb,
    /// Generate OpenGraph images for the specified crates
    GenerateOgImage {
        /// Crate names to generate OpenGraph images for
        #[arg(required = true)]
        names: Vec<String>,
    },
    IndexVersionDownloadsArchive,
    NormalizeIndex {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    ProcessCdnLogQueue(jobs::ProcessCdnLogQueue),
    SendTokenExpiryNotifications,
    SquashIndex,
    SyncAdmins {
        /// Force a sync even if one is already in progress
        #[arg(long)]
        force: bool,
    },
    SyncCratesFeed,
    SyncUpdatesFeed,
    TrustpubCleanup,
    UpdateDownloads,
}

pub async fn run(command: Command) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;
    println!("Enqueueing background job: {command:?}");

    match command {
        Command::ArchiveVersionDownloads { before } => {
            before
                .map(jobs::ArchiveVersionDownloads::before)
                .unwrap_or_default()
                .enqueue(&mut conn)
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

            jobs::CheckTyposquat::new(&name).enqueue(&mut conn).await?;
        }
        Command::CleanProcessedLogFiles => {
            jobs::CleanProcessedLogFiles.enqueue(&mut conn).await?;
        }
        Command::DailyDbMaintenance => {
            jobs::DailyDbMaintenance.enqueue(&mut conn).await?;
        }
        Command::DumpDb => {
            jobs::DumpDb.enqueue(&mut conn).await?;
        }
        Command::GenerateOgImage { names } => {
            for name in names {
                jobs::GenerateOgImage::new(name).enqueue(&mut conn).await?;
            }
        }
        Command::IndexVersionDownloadsArchive => {
            jobs::IndexVersionDownloadsArchive
                .enqueue(&mut conn)
                .await?;
        }
        Command::NormalizeIndex { dry_run } => {
            jobs::NormalizeIndex::new(dry_run)
                .enqueue(&mut conn)
                .await?;
        }
        Command::ProcessCdnLogQueue(job) => {
            job.enqueue(&mut conn).await?;
        }
        Command::SendTokenExpiryNotifications => {
            jobs::SendTokenExpiryNotifications
                .enqueue(&mut conn)
                .await?;
        }
        Command::SquashIndex => {
            jobs::SquashIndex.enqueue(&mut conn).await?;
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

            jobs::SyncAdmins.enqueue(&mut conn).await?;
        }
        Command::SyncCratesFeed => {
            jobs::rss::SyncCratesFeed.enqueue(&mut conn).await?;
        }
        Command::SyncUpdatesFeed => {
            jobs::rss::SyncUpdatesFeed.enqueue(&mut conn).await?;
        }
        Command::TrustpubCleanup => {
            let job = jobs::trustpub::DeleteExpiredTokens;
            job.enqueue(&mut conn).await?;

            let job = jobs::trustpub::DeleteExpiredJtis;
            job.enqueue(&mut conn).await?;
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
                jobs::UpdateDownloads.enqueue(&mut conn).await?;
            }
        }
    };

    Ok(())
}
