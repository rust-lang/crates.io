use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crate::OkBool;

fn is_following(crate_name: &str, user: &impl RequestHelper) -> bool {
    #[derive(Deserialize)]
    struct F {
        following: bool,
    }

    user.get::<F>(&format!("/api/v1/crates/{crate_name}/following"))
        .good()
        .following
}

fn follow(crate_name: &str, user: &impl RequestHelper) {
    assert!(
        user.put::<OkBool>(&format!("/api/v1/crates/{crate_name}/follow"), b"" as &[u8])
            .good()
            .ok
    );
}

fn unfollow(crate_name: &str, user: &impl RequestHelper) {
    assert!(
        user.delete::<OkBool>(&format!("/api/v1/crates/{crate_name}/follow"))
            .good()
            .ok
    );
}

#[test]
fn following() {
    // TODO: Test anon requests as well?
    let (app, _, user) = TestApp::init().with_user();

    let crate_name = "foo_following";
    app.db(|conn| {
        CrateBuilder::new(crate_name, user.as_model().id).expect_build(conn);
    });

    assert!(!is_following(crate_name, &user));
    follow(crate_name, &user);
    follow(crate_name, &user);
    assert!(is_following(crate_name, &user));
    assert_eq!(user.search("following=1").crates.len(), 1);

    unfollow(crate_name, &user);
    unfollow(crate_name, &user);
    assert!(!is_following(crate_name, &user));
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
    assert!(is_following(crate_to_follow, &user));
    assert!(!is_following(crate_not_followed, &user));

    let json = token.search("following=1");
    assert_eq!(json.crates.len(), 1);
}
