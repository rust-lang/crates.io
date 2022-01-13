use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crate::OkBool;

#[test]
fn diesel_not_found_results_in_404() {
    let (_, _, user) = TestApp::init().with_user();

    user.get("/api/v1/crates/foo_following/following")
        .assert_not_found();
}

#[test]
fn following() {
    // TODO: Test anon requests as well?
    let (app, _, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("foo_following", user.as_model().id).expect_build(conn);
    });

    let is_following = || -> bool {
        #[derive(Deserialize)]
        struct F {
            following: bool,
        }

        user.get::<F>("/api/v1/crates/foo_following/following")
            .good()
            .following
    };

    let follow = || {
        assert!(
            user.put::<OkBool>("/api/v1/crates/foo_following/follow", b"")
                .good()
                .ok
        );
    };

    let unfollow = || {
        assert!(
            user.delete::<OkBool>("api/v1/crates/foo_following/follow")
                .good()
                .ok
        );
    };

    assert!(!is_following());
    follow();
    follow();
    assert!(is_following());
    assert_eq!(user.search("following=1").crates.len(), 1);

    unfollow();
    unfollow();
    assert!(!is_following());
    assert_eq!(user.search("following=1").crates.len(), 0);
}

#[test]
fn disallow_api_token_auth_for_get_crate_following_status() {
    let (app, _, _, token) = TestApp::init().with_token();
    let api_token = token.as_model();

    let a_crate = "a_crate";

    app.db(|conn| {
        CrateBuilder::new(a_crate, api_token.user_id).expect_build(conn);
    });

    // Token auth on GET for get following status is disallowed
    token
        .get(&format!("/api/v1/crates/{a_crate}/following"))
        .assert_forbidden();
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

    let is_following = |crate_name: &str| -> bool {
        #[derive(Deserialize)]
        struct F {
            following: bool,
        }

        // Token auth on GET for get following status is disallowed
        user.get::<F>(&format!("/api/v1/crates/{crate_name}/following"))
            .good()
            .following
    };

    let follow = |crate_name: &str| {
        assert!(
            token
                .put::<OkBool>(&format!("/api/v1/crates/{crate_name}/follow"), b"")
                .good()
                .ok
        );
    };

    follow(crate_to_follow);

    assert!(is_following(crate_to_follow));
    assert!(!is_following(crate_not_followed));

    let json = token.search("following=1");
    assert_eq!(json.crates.len(), 1);
}
