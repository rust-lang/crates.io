use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
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
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let conn = env.deadpool.get().await?;
        conn.interact(move |conn| {
            info!("Running VACUUM on version_downloads table");
            sql_query("VACUUM version_downloads;").execute(conn)?;
            info!("Finished running VACUUM on version_downloads table");
            Ok(())
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))?
    }
}
