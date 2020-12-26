//! Checks for any invariants we expect to be true, and pages whoever is on call
//! if they are not.
//!
//! Usage:
//!     cargo run --bin monitor

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use cargo_on_call::Event;
use cargo_registry::{db, schema::*};
use diesel::prelude::*;

fn main() -> Result<()> {
    let conn = db::connect_now()?;

    check_failing_background_jobs(&conn)?;
    check_stalled_update_downloads(&conn)?;
    check_spam_attack(&conn)?;
    Ok(())
}

/// Check for old background jobs that are not currently running.
///
/// This check includes `skip_locked` in the query and will only trigger on
/// enqueued jobs that have attempted to run and have failed (and are in the
/// queue awaiting a retry).
///
/// Within the default 15 minute time, a job should have already had several
/// failed retry attempts.
fn check_failing_background_jobs(conn: &PgConnection) -> Result<()> {
    use cargo_registry::schema::background_jobs::dsl::*;
    use diesel::dsl::*;
    use diesel::sql_types::Integer;

    const EVENT_KEY: &str = "background_jobs";

    println!("Checking for failed background jobs");

    // Max job execution time in minutes
    let max_job_time = dotenv::var("MAX_JOB_TIME")
        .map(|s| s.parse::<i32>().unwrap())
        .unwrap_or(15);

    let stalled_jobs: Vec<i32> = background_jobs
        .select(1.into_sql::<Integer>())
        .filter(created_at.lt(now - max_job_time.minutes()))
        .for_update()
        .skip_locked()
        .load(conn)?;

    let stalled_job_count = stalled_jobs.len();

    let event = if stalled_job_count > 0 {
        Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!(
                "{} jobs have been in the queue for more than {} minutes",
                stalled_job_count, max_job_time
            ),
        }
    } else {
        Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No stalled background jobs".into()),
        }
    };

    log_and_trigger_event(event)?;
    Ok(())
}

/// Check for an `update_downloads` job that has run longer than expected
fn check_stalled_update_downloads(conn: &PgConnection) -> Result<()> {
    use cargo_registry::schema::background_jobs::dsl::*;
    use chrono::{DateTime, NaiveDateTime, Utc};

    const EVENT_KEY: &str = "update_downloads_stalled";

    println!("Checking for stalled background jobs");

    // Max job execution time in minutes
    let max_job_time = dotenv::var("MONITOR_MAX_UPDATE_DOWNLOADS_TIME")
        .map(|s| s.parse::<u32>().unwrap() as i64)
        .unwrap_or(120);

    let start_time: Result<NaiveDateTime, _> = background_jobs
        .filter(job_type.eq("update_downloads"))
        .select(created_at)
        .first(conn);

    if let Ok(start_time) = start_time {
        let start_time = DateTime::<Utc>::from_utc(start_time, Utc);
        let minutes = Utc::now().signed_duration_since(start_time).num_minutes();

        if minutes > max_job_time {
            return log_and_trigger_event(Event::Trigger {
                incident_key: Some(EVENT_KEY.into()),
                description: format!("update_downloads job running for {} minutes", minutes),
            });
        }
    };

    log_and_trigger_event(Event::Resolve {
        incident_key: EVENT_KEY.into(),
        description: Some("No stalled update_downloads job".into()),
    })
}

/// Check for known spam patterns
fn check_spam_attack(conn: &PgConnection) -> Result<()> {
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

    let bad_crate: Option<String> = crates::table
        .filter(canon_crate_name(crates::name).eq(any(bad_crate_names)))
        .select(crates::name)
        .first(conn)
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
    let bad_author: Option<String> = query.first(conn).optional()?;

    if let Some(bad_author) = bad_author {
        event_description = Some(format!("Crate with author {} published", bad_author));
    }

    let event = if let Some(event_description) = event_description {
        Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!("{}, possible spam attack underway", event_description,),
        }
    } else {
        Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No spam crates detected".into()),
        }
    };

    log_and_trigger_event(event)?;
    Ok(())
}

fn log_and_trigger_event(event: Event) -> Result<()> {
    match event {
        Event::Trigger {
            ref description, ..
        } => println!("Paging on-call: {}", description),
        Event::Resolve {
            description: Some(ref description),
            ..
        } => println!("{}", description),
        _ => {} // noop
    }
    event.send()
}
