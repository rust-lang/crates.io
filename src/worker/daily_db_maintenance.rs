use crate::background_jobs::{BackgroundJob, Environment, PerformState};
use crate::swirl::PerformError;
use diesel::{sql_query, RunQueryDsl};

#[derive(Serialize, Deserialize)]
pub struct DailyDbMaintenanceJob;

impl BackgroundJob for DailyDbMaintenanceJob {
    const JOB_NAME: &'static str = "daily_db_maintenance";

    /// Run daily database maintenance tasks
    ///
    /// By default PostgreSQL will run an auto-vacuum when 20% of the tuples in a table are dead.
    /// Because the `version_downloads` table includes years of historical data, we can accumulate
    /// a *lot* of garbage before an auto-vacuum is run.
    ///
    /// We only need to keep 90 days of entries in `version_downloads`. Once we have a mechanism to
    /// archive daily download counts and drop historical data, we can drop this task and rely on
    /// auto-vacuum again.
    fn run(&self, state: PerformState<'_>, _env: &Environment) -> Result<(), PerformError> {
        let mut conn = state.fresh_connection()?;

        info!("Running VACUUM on version_downloads table");
        sql_query("VACUUM version_downloads;").execute(&mut conn)?;
        info!("Finished running VACUUM on version_downloads table");
        Ok(())
    }
}
