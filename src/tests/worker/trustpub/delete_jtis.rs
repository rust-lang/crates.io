use crate::tests::util::TestApp;
use crate::worker::jobs::trustpub::DeleteExpiredJtis;
use chrono::{TimeDelta, Utc};
use crates_io_database::models::trustpub::NewUsedJti;
use crates_io_database::schema::trustpub_used_jtis;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
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
