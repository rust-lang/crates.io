use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

async fn assert_is_following(crate_name: &str, expected: bool, user: &impl RequestHelper) {
    let response = user
        .get::<()>(&format!("/api/v1/crates/{crate_name}/following"))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json(), json!({ "following": expected }));
}

async fn follow(crate_name: &str, user: &impl RequestHelper) {
    let response = user
        .put::<()>(&format!("/api/v1/crates/{crate_name}/follow"), b"" as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json(), json!({ "ok": true }));
}

async fn unfollow(crate_name: &str, user: &impl RequestHelper) {
    let response = user
        .delete::<()>(&format!("/api/v1/crates/{crate_name}/follow"))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json(), json!({ "ok": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated_requests() {
    const CRATE_NAME: &str = "foo";

    let (app, anon, user) = TestApp::init().with_user();
    let mut conn = app.db_conn();

    CrateBuilder::new(CRATE_NAME, user.as_model().id).expect_build(&mut conn);

    let response = anon
        .get::<()>(&format!("/api/v1/crates/{CRATE_NAME}/following"))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);

    let response = anon
        .put::<()>(&format!("/api/v1/crates/{CRATE_NAME}/follow"), b"" as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);

    let response = anon
        .delete::<()>(&format!("/api/v1/crates/{CRATE_NAME}/follow"))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_following() {
    const CRATE_NAME: &str = "foo_following";

    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.db_conn();

    CrateBuilder::new(CRATE_NAME, user.as_model().id).expect_build(&mut conn);

    // Check that initially we are not following the crate yet.
    assert_is_following(CRATE_NAME, false, &user).await;

    // Follow the crate and check that we are now following it.
    follow(CRATE_NAME, &user).await;
    assert_is_following(CRATE_NAME, true, &user).await;
    assert_that!(user.search("following=1").await.crates, len(eq(1)));

    // Follow the crate again and check that we are still following it
    // (aka. the request is idempotent).
    follow(CRATE_NAME, &user).await;
    assert_is_following(CRATE_NAME, true, &user).await;

    // Unfollow the crate and check that we are not following it anymore.
    unfollow(CRATE_NAME, &user).await;
    assert_is_following(CRATE_NAME, false, &user).await;
    assert_that!(user.search("following=1").await.crates, empty());

    // Unfollow the crate again and check that this call is also idempotent.
    unfollow(CRATE_NAME, &user).await;
    assert_is_following(CRATE_NAME, false, &user).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (_, _, user) = TestApp::init().with_user();

    let response = user
        .get::<()>("/api/v1/crates/unknown-crate/following")
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown-crate` does not exist"}]}"###);

    let response = user
        .put::<()>("/api/v1/crates/unknown-crate/follow", b"" as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown-crate` does not exist"}]}"###);

    let response = user
        .delete::<()>("/api/v1/crates/unknown-crate/follow")
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown-crate` does not exist"}]}"###);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_api_token_auth() {
    const CRATE_TO_FOLLOW: &str = "some_crate_to_follow";
    const CRATE_NOT_TO_FOLLOW: &str = "another_crate";

    let (app, _, user, token) = TestApp::init().with_token();
    let mut conn = app.db_conn();
    let api_token = token.as_model();

    CrateBuilder::new(CRATE_TO_FOLLOW, api_token.user_id).expect_build(&mut conn);
    CrateBuilder::new(CRATE_NOT_TO_FOLLOW, api_token.user_id).expect_build(&mut conn);

    follow(CRATE_TO_FOLLOW, &token).await;

    // Token auth on GET for get following status is disallowed
    assert_is_following(CRATE_TO_FOLLOW, true, &user).await;
    assert_is_following(CRATE_NOT_TO_FOLLOW, false, &user).await;

    let json = token.search("following=1").await;
    assert_that!(json.crates, len(eq(1)));
}
