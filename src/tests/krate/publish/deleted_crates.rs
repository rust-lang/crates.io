use crate::models::NewDeletedCrate;
use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use chrono::{Duration, Utc};
use crates_io_database::schema::deleted_crates;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_recently_deleted_crate_with_same_name() -> anyhow::Result<()> {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let now = Utc::now();
    let created_at = now - Duration::hours(24);
    let deleted_at = now - Duration::hours(1);
    let available_at = "2099-12-25T12:34:56Z".parse()?;

    let deleted_crate = NewDeletedCrate::builder("actix_web")
        .created_at(&created_at)
        .deleted_at(&deleted_at)
        .available_at(&available_at)
        .build();

    diesel::insert_into(deleted_crates::table)
        .values(deleted_crate)
        .execute(&mut conn)
        .await?;

    let crate_to_publish = PublishBuilder::new("actix-web", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"A crate with the name `actix_web` was recently deleted. Reuse of this name will be available after 2099-12-25T12:34:56Z."}]}"#);
    assert_that!(app.stored_files().await, empty());

    Ok(())
}
