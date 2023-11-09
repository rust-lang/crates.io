use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

fn assert_is_following(crate_name: &str, expected: bool, user: &impl RequestHelper) {
    let response = user.get::<()>(&format!("/api/v1/crates/{crate_name}/following"));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_json(), json!({ "following": expected }));
}

fn follow(crate_name: &str, user: &impl RequestHelper) {
    let response = user.put::<()>(&format!("/api/v1/crates/{crate_name}/follow"), b"" as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_json(), json!({ "ok": true }));
}

fn unfollow(crate_name: &str, user: &impl RequestHelper) {
    let response = user.delete::<()>(&format!("/api/v1/crates/{crate_name}/follow"));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.into_json(), json!({ "ok": true }));
}

#[test]
fn following() {
    // TODO: Test anon requests as well?
    let (app, _, user) = TestApp::init().with_user();

    let crate_name = "foo_following";
    app.db(|conn| {
        CrateBuilder::new(crate_name, user.as_model().id).expect_build(conn);
    });

    assert_is_following(crate_name, false, &user);
    follow(crate_name, &user);
    follow(crate_name, &user);
    assert_is_following(crate_name, true, &user);
    assert_eq!(user.search("following=1").crates.len(), 1);

    unfollow(crate_name, &user);
    unfollow(crate_name, &user);
    assert_is_following(crate_name, false, &user);
    assert_eq!(user.search("following=1").crates.len(), 0);
}

#[test]
fn getting_followed_crates_allows_api_token_auth() {
    let (app, _, user, token) = TestApp::init().with_token();
    let api_token = token.as_model();

    let crate_to_follow = "some_crate_to_follow";
    let crate_not_followed = "another_crate";

    app.db(|conn| {
        CrateBuilder::new(crate_to_follow, api_token.user_id).expect_build(conn);
        CrateBuilder::new(crate_not_followed, api_token.user_id).expect_build(conn);
    });

    follow(crate_to_follow, &token);

    // Token auth on GET for get following status is disallowed
    assert_is_following(crate_to_follow, true, &user);
    assert_is_following(crate_not_followed, false, &user);

    let json = token.search("following=1");
    assert_eq!(json.crates.len(), 1);
}
