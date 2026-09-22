use crate::schema::{background_jobs, crates, versions};
use crate::util::errors::AppResult;
use chrono::Duration;
use diesel::dsl::{count_star, min, now};
use diesel::prelude::*;
use diesel::sql_types::Interval;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::collections::HashMap;

/// A complete snapshot of service-level metrics queried from the database.
#[derive(Debug)]
pub struct ServiceMetricsSnapshot {
    /// Number of crates ever published.
    pub crates_total: i64,

    /// Number of versions ever published.
    pub versions_total: i64,

    /// Outstanding background jobs grouped by `(priority, job)`.
    pub background_jobs: HashMap<(String, String), BackgroundJobStats>,
}

/// Measurements of outstanding jobs in one job type and priority group.
#[derive(Debug)]
pub struct BackgroundJobStats {
    /// Number of jobs, including those currently running.
    pub count: i64,

    /// Time since the oldest job was enqueued, negative for future timestamps.
    pub oldest_age: Duration,
}

impl ServiceMetricsSnapshot {
    /// Loads a complete snapshot from the database.
    pub async fn load(conn: &mut AsyncPgConnection) -> AppResult<Self> {
        let crates_total = crates::table.select(count_star()).first(conn).await?;
        let versions_total = versions::table.select(count_star()).first(conn).await?;

        // Diesel only implements timestamp-minus-interval subtraction directly.
        diesel::infix_operator!(TimestampDifference, " - ", Interval, backend: diesel::pg::Pg);
        let oldest_created_at = min(background_jobs::created_at).assume_not_null();
        let oldest_age = TimestampDifference::new(now, oldest_created_at);

        let queued_jobs = background_jobs::table
            .group_by((background_jobs::job_type, background_jobs::priority))
            .select((
                background_jobs::job_type,
                background_jobs::priority,
                count_star(),
                oldest_age,
            ))
            .load::<(String, i16, i64, Duration)>(conn)
            .await?;

        let background_jobs = queued_jobs
            .into_iter()
            .map(|(job, priority, count, oldest_age)| {
                let stats = BackgroundJobStats { count, oldest_age };
                ((priority.to_string(), job), stats)
            })
            .collect();

        Ok(Self {
            crates_total,
            versions_total,
            background_jobs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;
    use diesel_async::AsyncConnection;

    #[tokio::test]
    async fn loads_background_job_ages() -> AppResult<()> {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let snapshot = ServiceMetricsSnapshot::load(&mut conn).await?;
        assert!(snapshot.background_jobs.is_empty());

        let jobs = "INSERT INTO background_jobs (job_type, priority, data, created_at) VALUES
             ('job_a', 0, '{}', now() - interval '60.5 seconds'),
             ('job_a', 0, '{}', now() - interval '10 seconds'),
             ('job_a', -1, '{}', now() - interval '120 seconds'),
             ('job_b', 0, '{}', now() - interval '2 days'),
             ('job_b', 10, '{}', now() + interval '1 hour')";

        let snapshot = conn
            .transaction(async |conn| {
                diesel::sql_query(jobs).execute(conn).await?;
                ServiceMetricsSnapshot::load(conn).await
            })
            .await?;

        assert_eq!(snapshot.background_jobs.len(), 4);

        for (priority, job, count, age_ms) in [
            ("0", "job_a", 2, 60_500),
            ("-1", "job_a", 1, 120_000),
            ("0", "job_b", 1, 172_800_000),
            ("10", "job_b", 1, -3_600_000),
        ] {
            let queue = &snapshot.background_jobs[&(priority.into(), job.into())];
            assert_eq!(queue.count, count);
            assert_eq!(queue.oldest_age, Duration::milliseconds(age_ms));
        }

        Ok(())
    }
}
