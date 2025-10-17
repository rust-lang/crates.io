use crate::worker::Environment;
use crates_io_database::schema::trustpub_used_jtis;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A background job that deletes expired JSON Web Token IDs (JTIs)
/// tokens from the database.
#[derive(Deserialize, Serialize)]
pub struct DeleteExpiredJtis;

impl BackgroundJob for DeleteExpiredJtis {
    const JOB_NAME: &'static str = "trustpub::delete_expired_jtis";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let mut conn = ctx.deadpool.get().await?;

        diesel::delete(trustpub_used_jtis::table)
            .filter(trustpub_used_jtis::expires_at.lt(diesel::dsl::now))
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
