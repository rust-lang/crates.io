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
