use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn diesel_not_found_results_in_404() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user
        .get::<()>("/api/v1/crates/foo_following/following")
        .await;
    assert_snapshot!(response.status(), @"404 Not Found");
}

#[tokio::test(flavor = "multi_thread")]
async fn disallow_api_token_auth_for_get_crate_following_status() {
    let (app, _, _, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let api_token = token.as_model();

    let a_crate = "a_crate";

    CrateBuilder::new(a_crate, api_token.user_id)
        .expect_build(&mut conn)
        .await;

    // Token auth on GET for get following status is disallowed
    let response = token
        .get::<()>(&format!("/api/v1/crates/{a_crate}/following"))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
}
