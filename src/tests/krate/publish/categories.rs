use crate::builders::PublishBuilder;
use crate::new_category;
use crate::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn good_categories() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_good_cat", "1.0.0").category("cat1");
    let response = token.async_publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn ignored_categories() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_ignored_cat", "1.0.0").category("bar");
    let response = token.async_publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_categories() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .async_publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .category("one")
                .category("two")
                .category("three")
                .category("four")
                .category("five")
                .category("six"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}
