use crate::tests::new_category;
use crate::tests::util::{RequestHelper, TestApp};
use insta::assert_json_snapshot;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread")]
async fn index() {
    let (app, anon) = TestApp::init().empty();
    let mut conn = app.db_conn();

    // List 0 categories if none exist
    let json: Value = anon.get("/api/v1/categories").await.good();
    assert_json_snapshot!(json);

    // Create a category and a subcategory
    new_category("foo", "foo", "Foo crates")
        .create_or_update(&mut conn)
        .unwrap();
    new_category("foo::bar", "foo::bar", "Bar crates")
        .create_or_update(&mut conn)
        .unwrap();

    // Only the top-level categories should be on the page
    let json: Value = anon.get("/api/v1/categories").await.good();
    assert_json_snapshot!(json, {
        ".categories[].created_at" => "[datetime]",
    });
}
