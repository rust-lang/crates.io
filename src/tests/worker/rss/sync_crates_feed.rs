use crate::util::TestApp;
use chrono::DateTime;
use crates_io::schema::crates;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::{PgConnection, RunQueryDsl};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_crates_feed() {
    let (app, _) = TestApp::full().empty();

    app.db(|conn| {
        create_crate(
            conn,
            "foo",
            Some("something something foo"),
            "2024-06-20T10:13:54Z",
        );
        create_crate(conn, "bar", None, "2024-06-20T12:45:12Z");
        create_crate(
            conn,
            "baz",
            Some("does it handle XML? <item>"),
            "2024-06-21T17:01:33Z",
        );
        create_crate(conn, "quux", None, "2024-06-21T17:03:45Z");

        jobs::rss::SyncCratesFeed.enqueue(conn).unwrap();
    });

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.stored_files().await.join("\n"), @"rss/crates.xml");

    let store = app.as_inner().storage.as_inner();
    let result = store.get(&"rss/crates.xml".into()).await.unwrap();
    let bytes = result.bytes().await.unwrap();
    let content = String::from_utf8(bytes.to_vec()).unwrap();
    assert_snapshot!(content);
}

fn create_crate(
    conn: &mut PgConnection,
    name: &str,
    description: Option<&str>,
    publish_time: &str,
) {
    let publish_time = DateTime::parse_from_rfc3339(publish_time)
        .unwrap()
        .naive_utc();

    diesel::insert_into(crates::table)
        .values((
            crates::name.eq(name),
            crates::description.eq(description),
            crates::created_at.eq(publish_time),
            crates::updated_at.eq(publish_time),
        ))
        .execute(conn)
        .unwrap();
}
