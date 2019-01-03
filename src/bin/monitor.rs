//! Checks for any invariants we expect to be true, and pages whoever is on call
//! if they are not.
//!
//! Usage:
//!     cargo run --bin monitor

#![deny(warnings)]

#[macro_use]
extern crate serde_derive;

mod on_call;

use cargo_registry::{db, util::CargoResult};
use diesel::prelude::*;
use std::env;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;

    check_stalled_background_jobs(&conn)?;
    Ok(())
}

fn check_stalled_background_jobs(conn: &PgConnection) -> CargoResult<()> {
    use cargo_registry::schema::background_jobs::dsl::*;
    use diesel::dsl::*;

    const BACKGROUND_JOB_KEY: &str = "background_jobs";

    println!("Checking for stalled background jobs");

    let max_job_time = env::var("MAX_JOB_TIME")
        .map(|s| s.parse::<i32>().unwrap())
        .unwrap_or(15);

    let stalled_job_count = background_jobs
        .filter(created_at.lt(now - max_job_time.minutes()))
        .count()
        .get_result::<i64>(conn)?;

    let event = if stalled_job_count > 0 {
        on_call::Event::Trigger {
            incident_key: Some(BACKGROUND_JOB_KEY.into()),
            description: format!(
                "{} jobs have been in the queue for more than {} minutes",
                stalled_job_count, max_job_time
            ),
        }
    } else {
        on_call::Event::Resolve {
            incident_key: BACKGROUND_JOB_KEY.into(),
            description: Some("No stalled background jobs".into()),
        }
    };

    log_and_trigger_event(event)?;
    Ok(())
}

fn log_and_trigger_event(event: on_call::Event) -> CargoResult<()> {
    match event {
        on_call::Event::Trigger {
            ref description, ..
        } => println!("Paging on-call: {}", description),
        on_call::Event::Resolve {
            description: Some(ref description),
            ..
        } => println!("{}", description),
        _ => {} // noop
    }
    event.send()
}
