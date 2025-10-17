use crate::builders::PublishBuilder;
use crate::new_category;
use crate::util::{RequestHelper, TestApp};
use crates_io_database::schema::categories;
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn good_categories() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    insert_into(categories::table)
        .values(new_category("Category 1", "cat1", "Category 1 crates"))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo_good_cat", "1.0.0").category("cat1");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn ignored_categories() {
    let (_, _, _, token) = TestApp::full().with_token().await;

    let crate_to_publish = PublishBuilder::new("foo_ignored_cat", "1.0.0").category("bar");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The following category slugs are not currently supported on crates.io: bar\n\nSee https://crates.io/category_slugs for a list of supported slugs."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_categories() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .category("one")
                .category("two")
                .category("three")
                .category("four")
                .category("five")
                .category("six"),
        )
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"expected at most 5 categories per crate"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}
