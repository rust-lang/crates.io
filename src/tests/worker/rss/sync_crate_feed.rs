use crate::schema::{crates, versions};
use crate::tests::util::TestApp;
use crate::worker::jobs;
use chrono::DateTime;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_crate_feed() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    create_version(&mut conn, "foo", "0.1.0", "2024-06-20T10:13:54Z").await?;
    create_version(&mut conn, "foo", "0.1.1", "2024-06-20T12:45:12Z").await?;
    create_version(&mut conn, "foo", "1.0.0", "2024-06-21T17:01:33Z").await?;
    create_version(&mut conn, "bar", "3.0.0-beta.1", "2024-06-21T17:03:45Z").await?;
    create_version(&mut conn, "foo", "1.1.0", "2024-06-22T08:30:01Z").await?;
    create_version(&mut conn, "foo", "1.2.0", "2024-06-22T15:57:19Z").await?;

    let job = jobs::rss::SyncCrateFeed::new("foo".to_string());
    job.enqueue(&mut conn).await?;

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.stored_files().await.join("\n"), @"rss/crates/foo.xml");

    let store = app.as_inner().storage.as_inner();
    let result = store.get(&"rss/crates/foo.xml".into()).await?;
    let bytes = result.bytes().await?;
    let content = String::from_utf8(bytes.to_vec())?;
    assert_snapshot!(content);

    Ok(())
}

async fn create_version(
    conn: &mut AsyncPgConnection,
    name: &str,
    version: &str,
    publish_time: &str,
) -> anyhow::Result<i32> {
    let publish_time = DateTime::parse_from_rfc3339(publish_time)?.naive_utc();

    let crate_id = crates::table
        .select(crates::id)
        .filter(crates::name.eq(name))
        .get_result::<i32>(conn)
        .await
        .optional()?;

    let crate_id = match crate_id {
        Some(crate_id) => crate_id,
        None => {
            diesel::insert_into(crates::table)
                .values((
                    crates::name.eq(name),
                    crates::created_at.eq(publish_time),
                    crates::updated_at.eq(publish_time),
                ))
                .returning(crates::id)
                .get_result(conn)
                .await?
        }
    };

    let version_id = diesel::insert_into(versions::table)
        .values((
            versions::crate_id.eq(crate_id),
            versions::num.eq(version),
            versions::num_no_build.eq(version),
            versions::created_at.eq(publish_time),
            versions::updated_at.eq(publish_time),
            versions::checksum.eq("checksum"),
            versions::crate_size.eq(0),
        ))
        .returning(versions::id)
        .get_result(conn)
        .await?;

    Ok(version_id)
}
