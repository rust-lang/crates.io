//! Checks for any invariants we expect to be true, and pages whoever is on call
//! if they are not.
//!
//! Usage:
//!     cargo run --bin monitor

#![deny(warnings, clippy::all, rust_2018_idioms)]

mod on_call;

use cargo_registry::{db, schema::*, util::CargoResult};
use diesel::prelude::*;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;

    check_stalled_background_jobs(&conn)?;
    check_spam_attack(&conn)?;
    Ok(())
}

fn check_stalled_background_jobs(conn: &PgConnection) -> CargoResult<()> {
    use cargo_registry::schema::background_jobs::dsl::*;
    use diesel::dsl::*;

    const EVENT_KEY: &str = "background_jobs";

    println!("Checking for stalled background jobs");

    let max_job_time = dotenv::var("MAX_JOB_TIME")
        .map(|s| s.parse::<i32>().unwrap())
        .unwrap_or(15);

    let stalled_job_count = background_jobs
        .filter(created_at.lt(now - max_job_time.minutes()))
        .count()
        .get_result::<i64>(conn)?;

    let event = if stalled_job_count > 0 {
        on_call::Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!(
                "{} jobs have been in the queue for more than {} minutes",
                stalled_job_count, max_job_time
            ),
        }
    } else {
        on_call::Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No stalled background jobs".into()),
        }
    };

    log_and_trigger_event(event)?;
    Ok(())
}

fn check_spam_attack(conn: &PgConnection) -> CargoResult<()> {
    use cargo_registry::models::krate::canon_crate_name;
    use diesel::dsl::*;
    use diesel::sql_types::Bool;

    const EVENT_KEY: &str = "spam_attack";

    println!("Checking for crates indicating someone is spamming us");

    let bad_crate_names = dotenv::var("SPAM_CRATE_NAMES");
    let bad_crate_names: Vec<_> = bad_crate_names
        .as_ref()
        .map(|s| s.split(',').collect())
        .unwrap_or_default();
    let bad_author_patterns = dotenv::var("SPAM_AUTHOR_PATTERNS");
    let bad_author_patterns: Vec<_> = bad_author_patterns
        .as_ref()
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let mut event_description = None;

    let bad_crate = crates::table
        .filter(canon_crate_name(crates::name).eq(any(bad_crate_names)))
        .select(crates::name)
        .first::<String>(conn)
        .optional()?;

    if let Some(bad_crate) = bad_crate {
        event_description = Some(format!("Crate named {} published", bad_crate));
    }

    let mut query = version_authors::table
        .select(version_authors::name)
        .filter(false.into_sql::<Bool>()) // Never return anything if we have no patterns
        .into_boxed();
    for author_pattern in bad_author_patterns {
        query = query.or_filter(version_authors::name.like(author_pattern));
    }
    let bad_author = query.first::<String>(conn).optional()?;

    if let Some(bad_author) = bad_author {
        event_description = Some(format!("Crate with author {} published", bad_author));
    }

    let event = if let Some(event_description) = event_description {
        on_call::Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!("{}, possible spam attack underway", event_description,),
        }
    } else {
        on_call::Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No spam crates detected".into()),
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
