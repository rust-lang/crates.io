use crate::schema::crates;
use crate::tests::util::TestApp;
use crate::worker::jobs;
use chrono::DateTime;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_crates_feed() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let description = Some("something something foo");
    create_crate(&mut conn, "foo", description, "2024-06-20T10:13:54Z").await?;
    create_crate(&mut conn, "bar", None, "2024-06-20T12:45:12Z").await?;
    let description = Some("does it handle XML? <item> ]]>");
    create_crate(&mut conn, "baz", description, "2024-06-21T17:01:33Z").await?;
    create_crate(&mut conn, "quux", None, "2024-06-21T17:03:45Z").await?;

    jobs::rss::SyncCratesFeed.enqueue(&mut conn).await?;

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.stored_files().await.join("\n"), @"rss/crates.xml");

    let store = app.as_inner().storage.as_inner();
    let result = store.get(&"rss/crates.xml".into()).await?;
    let bytes = result.bytes().await?;
    let content = String::from_utf8(bytes.to_vec())?;
    assert_snapshot!(content);

    Ok(())
}

async fn create_crate(
    conn: &mut AsyncPgConnection,
    name: &str,
    description: Option<&str>,
    publish_time: &str,
) -> anyhow::Result<()> {
    let publish_time = DateTime::parse_from_rfc3339(publish_time)?.naive_utc();

    diesel::insert_into(crates::table)
        .values((
            crates::name.eq(name),
            crates::description.eq(description),
            crates::created_at.eq(publish_time),
            crates::updated_at.eq(publish_time),
        ))
        .execute(conn)
        .await?;

    Ok(())
}
