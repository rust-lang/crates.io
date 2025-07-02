use crate::models::update_default_version;
use crate::schema::crates;
use crate::worker::Environment;
use crate::worker::jobs::GenerateOgImage;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
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
        let crate_name = update_default_version(crate_id, &mut conn).await.and(
            // Get the crate name for OG image generation
            crates::table
                .filter(crates::id.eq(crate_id))
                .select(crates::name)
                .first::<String>(&mut conn)
                .await,
        );
        if let Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            ..,
        )) = crate_name
        {
            warn!("Skipping update default version for crate for {crate_id}: no crate found");
            return Ok(());
        }

        // Generate OG image after updating default version
        let crate_name = crate_name?;
        info!("Enqueueing OG image generation for crate {crate_name}");
        GenerateOgImage::new(crate_name).enqueue(&mut conn).await?;

        Ok(())
    }
}
