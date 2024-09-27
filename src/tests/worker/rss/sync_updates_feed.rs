use crate::schema::{crates, versions};
use crate::tests::util::TestApp;
use crate::worker::jobs;
use chrono::DateTime;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::{PgConnection, RunQueryDsl};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_updates_feed() {
    let (app, _) = TestApp::full().empty();

    app.db(|conn| {
        let d = "let's try & break this <item>";

        create_version(conn, "foo", "0.1.0", None, "2024-06-20T10:13:54Z");
        create_version(conn, "foo", "0.1.1", None, "2024-06-20T12:45:12Z");
        create_version(conn, "foo", "1.0.0", None, "2024-06-21T17:01:33Z");
        create_version(conn, "bar", "3.0.0-beta.1", Some(d), "2024-06-21T17:03:45Z");
        create_version(conn, "foo", "1.1.0", None, "2024-06-22T08:30:01Z");
        create_version(conn, "foo", "1.2.0", None, "2024-06-22T15:57:19Z");

        jobs::rss::SyncUpdatesFeed.enqueue(conn).unwrap();
    });

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.stored_files().await.join("\n"), @"rss/updates.xml");

    let store = app.as_inner().storage.as_inner();
    let result = store.get(&"rss/updates.xml".into()).await.unwrap();
    let bytes = result.bytes().await.unwrap();
    let content = String::from_utf8(bytes.to_vec()).unwrap();
    assert_snapshot!(content);
}

fn create_version(
    conn: &mut PgConnection,
    name: &str,
    version: &str,
    description: Option<&str>,
    publish_time: &str,
) -> i32 {
    let publish_time = DateTime::parse_from_rfc3339(publish_time)
        .unwrap()
        .naive_utc();

    let crate_id = crates::table
        .select(crates::id)
        .filter(crates::name.eq(name))
        .get_result::<i32>(conn)
        .optional()
        .unwrap();

    let crate_id = crate_id.unwrap_or_else(|| {
        diesel::insert_into(crates::table)
            .values((
                crates::name.eq(name),
                crates::description.eq(description),
                crates::created_at.eq(publish_time),
                crates::updated_at.eq(publish_time),
            ))
            .returning(crates::id)
            .get_result(conn)
            .unwrap()
    });

    diesel::insert_into(versions::table)
        .values((
            versions::crate_id.eq(crate_id),
            versions::num.eq(version),
            versions::created_at.eq(publish_time),
            versions::updated_at.eq(publish_time),
            versions::checksum.eq("checksum"),
        ))
        .returning(versions::id)
        .get_result(conn)
        .unwrap()
}
