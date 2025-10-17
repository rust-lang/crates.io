use crate::worker::Environment;
use crates_io_database::schema::trustpub_tokens;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A background job that deletes expired temporary access
/// tokens from the database.
#[derive(Deserialize, Serialize)]
pub struct DeleteExpiredTokens;

impl BackgroundJob for DeleteExpiredTokens {
    const JOB_NAME: &'static str = "trustpub::delete_expired_tokens";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let mut conn = ctx.deadpool.get().await?;

        diesel::delete(trustpub_tokens::table)
            .filter(trustpub_tokens::expires_at.lt(diesel::dsl::now))
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
