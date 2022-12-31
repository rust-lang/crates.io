#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use cargo_registry::schema::background_jobs::dsl::*;
use cargo_registry::{db, worker};
use clap::Parser;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(name = "enqueue-job", rename_all = "snake_case")]
enum Command {
    UpdateDownloads,
    DumpDb {
        #[arg(env = "READ_ONLY_REPLICA_URL")]
        database_url: String,
        #[arg(default_value = "db-dump.tar.gz")]
        target_name: String,
    },
    DailyDbMaintenance,
    SquashIndex,
}

fn main() -> Result<()> {
    let command = Command::parse();

    let conn = db::oneoff_connection()?;
    println!("Enqueueing background job: {command:?}");

    match command {
        Command::UpdateDownloads => {
            let count: i64 = background_jobs
                .filter(job_type.eq("update_downloads"))
                .count()
                .get_result(&conn)
                .unwrap();

            if count > 0 {
                println!("Did not enqueue update_downloads, existing job already in progress");
                Ok(())
            } else {
                Ok(worker::update_downloads().enqueue(&conn)?)
            }
        }
        Command::DumpDb {
            database_url,
            target_name,
        } => Ok(worker::dump_db(database_url, target_name).enqueue(&conn)?),
        Command::DailyDbMaintenance => Ok(worker::daily_db_maintenance().enqueue(&conn)?),
        Command::SquashIndex => Ok(worker::squash_index().enqueue(&conn)?),
    }
}
