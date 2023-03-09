use crate::schema::background_jobs::dsl::*;
use crate::{db, worker};
use anyhow::Result;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(
    name = "enqueue-job",
    about = "Add a job to the background worker queue",
    rename_all = "snake_case"
)]
pub enum Command {
    UpdateDownloads,
    DumpDb {
        #[arg(env = "READ_ONLY_REPLICA_URL")]
        database_url: String,
        #[arg(default_value = "db-dump.tar.gz")]
        target_name: String,
    },
    DailyDbMaintenance,
    SquashIndex,
    NormalizeIndex {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    FixFeatures2 {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
}

pub fn run(command: Command) -> Result<()> {
    let conn = &mut db::oneoff_connection()?;
    println!("Enqueueing background job: {command:?}");

    match command {
        Command::UpdateDownloads => {
            let count: i64 = background_jobs
                .filter(job_type.eq("update_downloads"))
                .count()
                .get_result(conn)
                .unwrap();

            if count > 0 {
                println!("Did not enqueue update_downloads, existing job already in progress");
                Ok(())
            } else {
                Ok(worker::update_downloads().enqueue(conn)?)
            }
        }
        Command::DumpDb {
            database_url,
            target_name,
        } => Ok(worker::dump_db(database_url, target_name).enqueue(conn)?),
        Command::DailyDbMaintenance => Ok(worker::daily_db_maintenance().enqueue(conn)?),
        Command::SquashIndex => Ok(worker::squash_index().enqueue(conn)?),
        Command::NormalizeIndex { dry_run } => Ok(worker::normalize_index(dry_run).enqueue(conn)?),
        Command::FixFeatures2 { dry_run } => Ok(worker::fix_features2(dry_run).enqueue(conn)?),
    }
}
