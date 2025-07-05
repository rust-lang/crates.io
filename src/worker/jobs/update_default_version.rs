use crate::models::update_default_version;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Serialize, Deserialize)]
pub struct UpdateDefaultVersion {
    crate_id: i32,
}

impl UpdateDefaultVersion {
    pub fn new(crate_id: i32) -> Self {
        Self { crate_id }
    }
}

impl BackgroundJob for UpdateDefaultVersion {
    const JOB_NAME: &'static str = "update_default_version";
    const PRIORITY: i16 = 80;
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let crate_id = self.crate_id;

        info!("Updating default version for crate {crate_id}");
        let mut conn = ctx.deadpool.get().await?;
        let res = update_default_version(crate_id, &mut conn).await;
        if let Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            ..,
        )) = res
        {
            warn!("Skipping update default version for crate for {crate_id}: no crate found",);
            return Ok(());
        }

        res.map_err(|e| e.into())
    }
}
