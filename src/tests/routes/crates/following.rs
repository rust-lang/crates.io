use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};

#[tokio::test(flavor = "multi_thread")]
async fn diesel_not_found_results_in_404() {
    let (_, _, user) = TestApp::init().with_user().await;

    user.get("/api/v1/crates/foo_following/following")
        .await
        .assert_not_found();
}

#[tokio::test(flavor = "multi_thread")]
async fn disallow_api_token_auth_for_get_crate_following_status() {
    let (app, _, _, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn();
    let api_token = token.as_model();

    let a_crate = "a_crate";

    CrateBuilder::new(a_crate, api_token.user_id).expect_build(&mut conn);

    // Token auth on GET for get following status is disallowed
    token
        .get(&format!("/api/v1/crates/{a_crate}/following"))
        .await
        .assert_forbidden();
}
