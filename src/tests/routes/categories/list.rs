use crate::tests::new_category;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::schema::categories;
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use insta::assert_json_snapshot;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread")]
async fn index() {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.async_db_conn().await;

    // List 0 categories if none exist
    let json: Value = anon.get("/api/v1/categories").await.good();
    assert_json_snapshot!(json);

    // Create a category and a subcategory
    let cats = vec![
        new_category("foo", "foo", "Foo crates"),
        new_category("foo::bar", "foo::bar", "Bar crates"),
    ];

    insert_into(categories::table)
        .values(cats)
        .execute(&mut conn)
        .await
        .unwrap();

    // Only the top-level categories should be on the page
    let json: Value = anon.get("/api/v1/categories").await.good();
    assert_json_snapshot!(json, {
        ".categories[].created_at" => "[datetime]",
    });
}
