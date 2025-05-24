use crate::worker::Environment;
use crates_io_database::schema::trustpub_used_jtis;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::sync::Arc;

/// A background job that deletes expired JSON Web Token IDs (JTIs)
/// tokens from the database.
#[derive(Deserialize, Serialize)]
pub struct DeleteExpiredJtis;

impl BackgroundJob for DeleteExpiredJtis {
    const JOB_NAME: &'static str = "trustpub::delete_expired_jtis";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::TestApp;
    use chrono::{TimeDelta, Utc};
    use crates_io_database::models::trustpub::NewUsedJti;
    use insta::assert_compact_debug_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_expiry() -> anyhow::Result<()> {
        let (app, _client) = TestApp::full().empty().await;
        let mut conn = app.db_conn().await;

        let jti = NewUsedJti {
            expires_at: Utc::now() + TimeDelta::minutes(30),
            jti: "foo",
        };
        jti.insert(&mut conn).await?;

        let jti = NewUsedJti {
            expires_at: Utc::now() - TimeDelta::minutes(5),
            jti: "bar",
        };
        jti.insert(&mut conn).await?;

        DeleteExpiredJtis.enqueue(&mut conn).await?;
        app.run_pending_background_jobs().await;

        // Check that the expired token was deleted
        let known_jtis: Vec<String> = trustpub_used_jtis::table
            .select(trustpub_used_jtis::jti)
            .load(&mut conn)
            .await?;

        assert_compact_debug_snapshot!(known_jtis, @r#"["foo"]"#);

        Ok(())
    }
}
