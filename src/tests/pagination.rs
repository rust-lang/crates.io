use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn pagination_blocks_high_page_numbers() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.max_allowed_page_offset = 1;
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;
    let user = user.as_model();

    CrateBuilder::new("pagination_links_1", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_2", user.id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("pagination_links_3", user.id)
        .expect_build(&mut conn)
        .await;

    let response = anon
        .get_with_query::<()>("/api/v1/crates", "page=2&per_page=1")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Page 2 is unavailable for performance reasons. Please take a look at https://crates.io/data-access for alternatives."}]}"#);
}
