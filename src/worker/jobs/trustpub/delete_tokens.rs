use crate::worker::Environment;
use crates_io_database::schema::trustpub_tokens;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::TestApp;
    use chrono::{TimeDelta, Utc};
    use crates_io_database::models::trustpub::NewToken;
    use insta::assert_compact_debug_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_expiry() -> anyhow::Result<()> {
        let (app, _client) = TestApp::full().empty().await;
        let mut conn = app.db_conn().await;

        let token = NewToken {
            expires_at: Utc::now() + TimeDelta::minutes(30),
            hashed_token: &[0xC0, 0xFF, 0xEE],
            crate_ids: &[1],
        };
        token.insert(&mut conn).await?;

        let token = NewToken {
            expires_at: Utc::now() - TimeDelta::minutes(5),
            hashed_token: &[0xBA, 0xAD, 0xF0, 0x0D],
            crate_ids: &[2],
        };
        token.insert(&mut conn).await?;

        DeleteExpiredTokens.enqueue(&mut conn).await?;
        app.run_pending_background_jobs().await;

        // Check that the expired token was deleted
        let crate_ids: Vec<Vec<Option<i32>>> = trustpub_tokens::table
            .select(trustpub_tokens::crate_ids)
            .load(&mut conn)
            .await?;

        assert_compact_debug_snapshot!(crate_ids, @"[[Some(1)]]");

        Ok(())
    }
}
