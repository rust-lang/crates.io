use crate::swirl::PerformError;
/// Run daily database maintenance tasks
///
/// By default PostgreSQL will run an auto-vacuum when 20% of the tuples in a table are dead.
/// Because the `version_downloads` table includes years of historical data, we can accumulate
/// a *lot* of garbage before an auto-vacuum is run.
///
/// We only need to keep 90 days of entries in `version_downloads`. Once we have a mechanism to
/// archive daily download counts and drop historical data, we can drop this task and rely on
/// auto-vacuum again.
use diesel::{sql_query, PgConnection, RunQueryDsl};

pub(crate) fn perform_daily_db_maintenance(conn: &mut PgConnection) -> Result<(), PerformError> {
    info!("Running VACUUM on version_downloads table");
    sql_query("VACUUM version_downloads;").execute(conn)?;
    info!("Finished running VACUUM on version_downloads table");
    Ok(())
}
