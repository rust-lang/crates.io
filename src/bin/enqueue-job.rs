#![deny(clippy::all)]

use cargo_registry::{db, env, tasks, util::Error};
use diesel::prelude::*;
use swirl::schema::background_jobs::dsl::*;
use swirl::Job;

fn main() -> Result<(), Error> {
    let conn = db::connect_now()?;
    let mut args = std::env::args().skip(1);

    let job = args.next().unwrap_or_default();
    println!("Enqueueing background job: {}", job);

    match &*job {
        "update_downloads" => {
            let count: i64 = background_jobs
                .filter(job_type.eq("update_downloads"))
                .count()
                .get_result(&conn)
                .unwrap();

            if count > 0 {
                println!("Did not enqueue update_downloads, existing job already in progress");
                Ok(())
            } else {
                Ok(tasks::update_downloads().enqueue(&conn)?)
            }
        }
        "dump_db" => {
            let database_url = args.next().unwrap_or_else(|| env("READ_ONLY_REPLICA_URL"));
            let target_name = args
                .next()
                .unwrap_or_else(|| String::from("db-dump.tar.gz"));
            Ok(tasks::dump_db(database_url, target_name).enqueue(&conn)?)
        }
        other => Err(Error::from(format!("Unrecognized job type `{}`", other))),
    }
}
