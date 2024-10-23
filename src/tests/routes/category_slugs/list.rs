use crate::tests::new_category;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::schema::categories;
use diesel::insert_into;
use diesel_async::RunQueryDsl;
use insta::assert_json_snapshot;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread")]
async fn category_slugs_returns_all_slugs_in_alphabetical_order() {
    let (app, anon) = TestApp::init().empty();
    let mut conn = app.async_db_conn().await;

    let cats = vec![
        new_category("Foo", "foo", "For crates that foo"),
        new_category("Bar", "bar", "For crates that bar"),
    ];

    insert_into(categories::table)
        .values(cats)
        .execute(&mut conn)
        .await
        .unwrap();

    let response: Value = anon.get("/api/v1/category_slugs").await.good();
    assert_json_snapshot!(response);
}
