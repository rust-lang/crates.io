//! Checks for any invariants we expect to be true, and pages whoever is on call
//! if they are not.
//!
//! Usage:
//!     cargo run --bin monitor

use anyhow::Result;
use crates_io::worker::jobs;
use crates_io::{db, schema::*};
use crates_io_diesel_helpers::canon_crate_name;
use crates_io_env_vars::{required_var, var, var_parsed};
use crates_io_pagerduty as pagerduty;
use crates_io_pagerduty::PagerdutyClient;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::sql_types::Timestamptz;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[tokio::main]
async fn main() -> Result<()> {
    let api_token = required_var("PAGERDUTY_API_TOKEN")?.into();
    let service_key = required_var("PAGERDUTY_INTEGRATION_KEY")?;
    let client = PagerdutyClient::new(api_token, service_key);

    let conn = &mut db::oneoff_connection().await?;

    check_failing_background_jobs(conn, &client).await?;
    check_stalled_update_downloads(conn, &client).await?;
    check_spam_attack(conn, &client).await?;
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
async fn check_failing_background_jobs(
    conn: &mut AsyncPgConnection,
    pagerduty: &PagerdutyClient,
) -> Result<()> {
    use diesel::dsl::*;
    use diesel::sql_types::Integer;

    const EVENT_KEY: &str = "background_jobs";

    println!("Checking for failed background jobs");

    // Max job execution time in minutes
    let max_job_time = var_parsed("MAX_JOB_TIME")?.unwrap_or(15);

    let stalled_jobs: Vec<i32> = background_jobs::table
        .select(1.into_sql::<Integer>())
        .filter(
            background_jobs::created_at.lt(now.into_sql::<Timestamptz>() - max_job_time.minutes()),
        )
        .filter(background_jobs::priority.ge(0))
        .for_update()
        .skip_locked()
        .load(conn)
        .await?;

    let stalled_job_count = stalled_jobs.len();

    let event = if stalled_job_count > 0 {
        pagerduty::Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!(
                "{stalled_job_count} jobs have been in the queue for more than {max_job_time} minutes"
            ),
        }
    } else {
        pagerduty::Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No stalled background jobs".into()),
        }
    };

    log_and_trigger_event(pagerduty, event).await?;

    Ok(())
}

/// Check for an `update_downloads` job that has run longer than expected
async fn check_stalled_update_downloads(
    conn: &mut AsyncPgConnection,
    pagerduty: &PagerdutyClient,
) -> Result<()> {
    use chrono::{DateTime, Utc};

    const EVENT_KEY: &str = "update_downloads_stalled";

    println!("Checking for stalled background jobs");

    // Max job execution time in minutes
    let max_job_time = var_parsed("MONITOR_MAX_UPDATE_DOWNLOADS_TIME")?.unwrap_or(120);

    let start_time: Result<DateTime<Utc>, _> = background_jobs::table
        .filter(background_jobs::job_type.eq(jobs::UpdateDownloads::JOB_NAME))
        .select(background_jobs::created_at)
        .first(conn)
        .await;

    if let Ok(start_time) = start_time {
        let minutes = Utc::now().signed_duration_since(start_time).num_minutes();

        if minutes > max_job_time {
            return log_and_trigger_event(
                pagerduty,
                pagerduty::Event::Trigger {
                    incident_key: Some(EVENT_KEY.into()),
                    description: format!("update_downloads job running for {minutes} minutes"),
                },
            )
            .await;
        }
    };

    log_and_trigger_event(
        pagerduty,
        pagerduty::Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No stalled update_downloads job".into()),
        },
    )
    .await
}

/// Check for known spam patterns
async fn check_spam_attack(
    conn: &mut AsyncPgConnection,
    pagerduty: &PagerdutyClient,
) -> Result<()> {
    const EVENT_KEY: &str = "spam_attack";

    println!("Checking for crates indicating someone is spamming us");

    let bad_crate_names = var("SPAM_CRATE_NAMES")?;
    let bad_crate_names: Vec<_> = bad_crate_names
        .as_ref()
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let mut event_description = None;

    let bad_crate: Option<String> = crates::table
        .filter(canon_crate_name(crates::name).eq_any(bad_crate_names))
        .select(crates::name)
        .first(conn)
        .await
        .optional()?;

    if let Some(bad_crate) = bad_crate {
        event_description = Some(format!("Crate named {bad_crate} published"));
    }

    let event = if let Some(event_description) = event_description {
        pagerduty::Event::Trigger {
            incident_key: Some(EVENT_KEY.into()),
            description: format!("{event_description}, possible spam attack underway"),
        }
    } else {
        pagerduty::Event::Resolve {
            incident_key: EVENT_KEY.into(),
            description: Some("No spam crates detected".into()),
        }
    };

    log_and_trigger_event(pagerduty, event).await?;
    Ok(())
}

async fn log_and_trigger_event(pagerduty: &PagerdutyClient, event: pagerduty::Event) -> Result<()> {
    match event {
        pagerduty::Event::Trigger {
            ref description, ..
        } => println!("Paging on-call: {description}"),
        pagerduty::Event::Resolve {
            description: Some(ref description),
            ..
        } => println!("{description}"),
        _ => {} // noop
    }
    pagerduty.send(&event).await
}
