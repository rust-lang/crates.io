//! Checks for any invariants we expect to be true, and pages whoever is on call
//! if they are not.
//!
//! Usage:
//!     cargo run --bin monitor

use anyhow::Result;
use crates_io::worker::jobs;
use crates_io::{db, schema::*};
use crates_io_database::fns::canon_crate_name;
use crates_io_env_vars::{required_var, var, var_parsed};
use crates_io_pagerduty as pagerduty;
use crates_io_pagerduty::PagerdutyClient;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::sql_types::Timestamptz;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

/// Identifies an invariant evaluated by the monitor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CheckId {
    BackgroundJobs,
    UpdateDownloads,
    SpamAttack,
}

/// The outcome of evaluating a monitor invariant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CheckStatus {
    Healthy,
    Unhealthy,
}

/// A provider-independent monitor result.
#[derive(Debug, Eq, PartialEq)]
struct CheckResult {
    id: CheckId,
    status: CheckStatus,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let service_key = required_var("PAGERDUTY_INTEGRATION_KEY")?.into();
    let pagerduty = PagerdutyClient::new(service_key);

    let conn = &mut db::oneoff_connection().await?;

    let result = check_failing_background_jobs(conn).await?;
    report_to_pagerduty(&pagerduty, &result).await?;

    let result = check_stalled_update_downloads(conn).await?;
    report_to_pagerduty(&pagerduty, &result).await?;

    let result = check_spam_attack(conn).await?;
    report_to_pagerduty(&pagerduty, &result).await?;

    Ok(())
}

/// Checks for old background jobs that are not currently running.
///
/// This check includes `skip_locked` in the query and will only trigger on
/// enqueued jobs that have attempted to run and have failed (and are in the
/// queue awaiting a retry).
///
/// Within the default 15 minute time, a job should have already had several
/// failed retry attempts.
async fn check_failing_background_jobs(conn: &mut AsyncPgConnection) -> Result<CheckResult> {
    use diesel::dsl::*;
    use diesel::sql_types::Integer;

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

    let result = if stalled_job_count > 0 {
        CheckResult {
            id: CheckId::BackgroundJobs,
            status: CheckStatus::Unhealthy,
            message: format!(
                "{stalled_job_count} jobs have been in the queue for more than {max_job_time} minutes"
            ),
        }
    } else {
        CheckResult {
            id: CheckId::BackgroundJobs,
            status: CheckStatus::Healthy,
            message: "No stalled background jobs".into(),
        }
    };

    Ok(result)
}

/// Checks for an `update_downloads` job that has run longer than expected
async fn check_stalled_update_downloads(conn: &mut AsyncPgConnection) -> Result<CheckResult> {
    use chrono::{DateTime, Utc};

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
            return Ok(CheckResult {
                id: CheckId::UpdateDownloads,
                status: CheckStatus::Unhealthy,
                message: format!("update_downloads job running for {minutes} minutes"),
            });
        }
    };

    Ok(CheckResult {
        id: CheckId::UpdateDownloads,
        status: CheckStatus::Healthy,
        message: "No stalled update_downloads job".into(),
    })
}

/// Checks for known spam patterns
async fn check_spam_attack(conn: &mut AsyncPgConnection) -> Result<CheckResult> {
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

    let result = if let Some(event_description) = event_description {
        CheckResult {
            id: CheckId::SpamAttack,
            status: CheckStatus::Unhealthy,
            message: format!("{event_description}, possible spam attack underway"),
        }
    } else {
        CheckResult {
            id: CheckId::SpamAttack,
            status: CheckStatus::Healthy,
            message: "No spam crates detected".into(),
        }
    };

    Ok(result)
}

/// Converts a provider-independent result into a PagerDuty event.
fn pagerduty_event(result: &CheckResult) -> pagerduty::Event {
    let incident_key = match result.id {
        CheckId::BackgroundJobs => "background_jobs",
        CheckId::UpdateDownloads => "update_downloads_stalled",
        CheckId::SpamAttack => "spam_attack",
    };

    match result.status {
        CheckStatus::Healthy => pagerduty::Event::Resolve {
            incident_key: incident_key.into(),
            description: Some(result.message.clone()),
        },
        CheckStatus::Unhealthy => pagerduty::Event::Trigger {
            incident_key: Some(incident_key.into()),
            description: result.message.clone(),
        },
    }
}

/// Reports a monitor result to PagerDuty.
async fn report_to_pagerduty(pagerduty: &PagerdutyClient, result: &CheckResult) -> Result<()> {
    match result.status {
        CheckStatus::Healthy => println!("{}", result.message),
        CheckStatus::Unhealthy => println!("Paging on-call: {}", result.message),
    }

    let event = pagerduty_event(result);
    pagerduty.send(&event).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_json_snapshot;

    #[test]
    fn maps_results_to_pagerduty_events() {
        let results = [
            CheckResult {
                id: CheckId::BackgroundJobs,
                status: CheckStatus::Unhealthy,
                message: "background jobs unhealthy".into(),
            },
            CheckResult {
                id: CheckId::BackgroundJobs,
                status: CheckStatus::Healthy,
                message: "background jobs healthy".into(),
            },
            CheckResult {
                id: CheckId::UpdateDownloads,
                status: CheckStatus::Unhealthy,
                message: "update downloads unhealthy".into(),
            },
            CheckResult {
                id: CheckId::UpdateDownloads,
                status: CheckStatus::Healthy,
                message: "update downloads healthy".into(),
            },
            CheckResult {
                id: CheckId::SpamAttack,
                status: CheckStatus::Unhealthy,
                message: "spam attack detected".into(),
            },
            CheckResult {
                id: CheckId::SpamAttack,
                status: CheckStatus::Healthy,
                message: "no spam attack detected".into(),
            },
        ];
        let events: Vec<_> = results.iter().map(pagerduty_event).collect();

        assert_json_snapshot!(events, @r#"
        [
          {
            "event_type": "trigger",
            "incident_key": "background_jobs",
            "description": "background jobs unhealthy"
          },
          {
            "event_type": "resolve",
            "incident_key": "background_jobs",
            "description": "background jobs healthy"
          },
          {
            "event_type": "trigger",
            "incident_key": "update_downloads_stalled",
            "description": "update downloads unhealthy"
          },
          {
            "event_type": "resolve",
            "incident_key": "update_downloads_stalled",
            "description": "update downloads healthy"
          },
          {
            "event_type": "trigger",
            "incident_key": "spam_attack",
            "description": "spam attack detected"
          },
          {
            "event_type": "resolve",
            "incident_key": "spam_attack",
            "description": "no spam attack detected"
          }
        ]
        "#);
    }
}
