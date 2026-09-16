use crate::schema::{background_jobs, crates, versions};
use crate::util::errors::AppResult;
use diesel::{dsl::count_star, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::collections::HashMap;

/// A complete snapshot of service-level metrics queried from the database.
#[derive(Debug)]
pub struct ServiceMetricsSnapshot {
    /// Number of crates ever published.
    pub crates_total: i64,

    /// Number of versions ever published.
    pub versions_total: i64,

    /// Number of queued background jobs grouped by `(priority, job)`.
    pub background_jobs: HashMap<(String, String), i64>,
}

impl ServiceMetricsSnapshot {
    /// Loads a complete snapshot from the database.
    pub async fn load(conn: &mut AsyncPgConnection) -> AppResult<Self> {
        let crates_total = crates::table.select(count_star()).first(conn).await?;
        let versions_total = versions::table.select(count_star()).first(conn).await?;

        let queued_jobs = background_jobs::table
            .group_by((background_jobs::job_type, background_jobs::priority))
            .select((
                background_jobs::job_type,
                background_jobs::priority,
                count_star(),
            ))
            .load::<(String, i16, i64)>(conn)
            .await?;

        let background_jobs = queued_jobs
            .into_iter()
            .map(|(job, priority, count)| ((priority.to_string(), job), count))
            .collect();

        Ok(Self {
            crates_total,
            versions_total,
            background_jobs,
        })
    }
}
