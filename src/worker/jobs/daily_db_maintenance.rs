use crate::worker::swirl::{BackgroundJob, PerformError, PerformState};
use crate::worker::Environment;
use diesel::{sql_query, RunQueryDsl};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct DailyDbMaintenance;

impl BackgroundJob for DailyDbMaintenance {
    const JOB_NAME: &'static str = "daily_db_maintenance";

    type Context = Arc<Environment>;

    /// Run daily database maintenance tasks
    ///
    /// By default PostgreSQL will run an auto-vacuum when 20% of the tuples in a table are dead.
    /// Because the `version_downloads` table includes years of historical data, we can accumulate
    /// a *lot* of garbage before an auto-vacuum is run.
    ///
    /// We only need to keep 90 days of entries in `version_downloads`. Once we have a mechanism to
    /// archive daily download counts and drop historical data, we can drop this task and rely on
    /// auto-vacuum again.
    fn run(&self, _state: PerformState<'_>, env: &Self::Context) -> Result<(), PerformError> {
        let mut conn = env.connection_pool.get()?;

        info!("Running VACUUM on version_downloads table");
        sql_query("VACUUM version_downloads;").execute(&mut *conn)?;
        info!("Finished running VACUUM on version_downloads table");
        Ok(())
    }
}
