use diesel::dsl::now;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Interval};
use diesel::{delete, update};

use crate::schema::{self, background_jobs};

#[derive(Queryable, Identifiable, Debug, Clone)]
pub(super) struct BackgroundJob {
    pub(super) id: i64,
    pub(super) job_type: String,
    pub(super) data: serde_json::Value,
}

fn retriable() -> Box<dyn BoxableExpression<background_jobs::table, Pg, SqlType = Bool>> {
    use diesel::dsl::*;
    use schema::background_jobs::dsl::*;

    sql_function!(fn power(x: Integer, y: Integer) -> Integer);

    Box::new(last_retry.lt(now - 1.minute().into_sql::<Interval>() * power(2, retries)))
}

/// Finds the next job that is unlocked, and ready to be retried. If a row is
/// found, it will be locked.
pub(super) fn find_next_unlocked_job(conn: &mut PgConnection) -> QueryResult<BackgroundJob> {
    use schema::background_jobs::dsl::*;

    background_jobs
        .select((id, job_type, data))
        .filter(retriable())
        .order((priority.desc(), id))
        .for_update()
        .skip_locked()
        .first::<BackgroundJob>(conn)
}

/// The number of jobs that have failed at least once
pub(super) fn failed_job_count(conn: &mut PgConnection) -> QueryResult<i64> {
    use schema::background_jobs::dsl::*;

    background_jobs
        .count()
        .filter(retries.gt(0))
        .get_result(conn)
}

/// Deletes a job that has successfully completed running
pub(super) fn delete_successful_job(conn: &mut PgConnection, job_id: i64) -> QueryResult<()> {
    use schema::background_jobs::dsl::*;

    delete(background_jobs.find(job_id)).execute(conn)?;
    Ok(())
}

/// Marks that we just tried and failed to run a job.
///
/// Ignores any database errors that may have occurred. If the DB has gone away,
/// we assume that just trying again with a new connection will succeed.
pub(super) fn update_failed_job(conn: &mut PgConnection, job_id: i64) {
    use schema::background_jobs::dsl::*;

    let _ = update(background_jobs.find(job_id))
        .set((retries.eq(retries + 1), last_retry.eq(now)))
        .execute(conn);
}
