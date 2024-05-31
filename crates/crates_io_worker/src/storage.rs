use crate::schema::background_jobs;
use diesel::dsl::now;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Interval};
use diesel::{delete, update};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
pub(super) struct BackgroundJob {
    pub(super) id: i64,
    pub(super) job_type: String,
    pub(super) data: serde_json::Value,
}

fn retriable() -> Box<dyn BoxableExpression<background_jobs::table, Pg, SqlType = Bool>> {
    use diesel::dsl::*;

    define_sql_function!(fn power(x: Integer, y: Integer) -> Integer);

    Box::new(
        background_jobs::last_retry
            .lt(now - 1.minute().into_sql::<Interval>() * power(2, background_jobs::retries)),
    )
}

/// Finds the next job that is unlocked, and ready to be retried. If a row is
/// found, it will be locked.
pub(super) fn find_next_unlocked_job(
    conn: &mut PgConnection,
    job_types: &[String],
) -> QueryResult<BackgroundJob> {
    background_jobs::table
        .select(BackgroundJob::as_select())
        .filter(background_jobs::job_type.eq_any(job_types))
        .filter(retriable())
        .order((background_jobs::priority.desc(), background_jobs::id))
        .for_update()
        .skip_locked()
        .first::<BackgroundJob>(conn)
}

/// The number of jobs that have failed at least once
pub(super) fn failed_job_count(conn: &mut PgConnection) -> QueryResult<i64> {
    background_jobs::table
        .count()
        .filter(background_jobs::retries.gt(0))
        .get_result(conn)
}

/// Deletes a job that has successfully completed running
pub(super) fn delete_successful_job(conn: &mut PgConnection, job_id: i64) -> QueryResult<()> {
    delete(background_jobs::table.find(job_id)).execute(conn)?;
    Ok(())
}

/// Marks that we just tried and failed to run a job.
///
/// Ignores any database errors that may have occurred. If the DB has gone away,
/// we assume that just trying again with a new connection will succeed.
pub(super) fn update_failed_job(conn: &mut PgConnection, job_id: i64) {
    let _ = update(background_jobs::table.find(job_id))
        .set((
            background_jobs::retries.eq(background_jobs::retries + 1),
            background_jobs::last_retry.eq(now),
        ))
        .execute(conn);
}
