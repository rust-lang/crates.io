use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::models::Crate;
use crates_io::worker::jobs;
use diesel::prelude::*;
use http::StatusCode;

#[test]
fn index_smoke_test() {
    let (app, _, _, token) = TestApp::full().with_token();
    let upstream = app.upstream_index();

    // Add a new crate

    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.put::<()>("/api/v1/crates/new", body);
    assert_eq!(response.status(), StatusCode::OK);

    // Check that the git index is updated asynchronously
    assert_ok_eq!(upstream.list_commits(), vec!["Initial Commit"]);
    assert_ok_eq!(upstream.crate_exists("serde"), false);

    app.run_pending_background_jobs();
    assert_ok_eq!(
        upstream.list_commits(),
        vec!["Initial Commit", "Create crate `serde`"]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), true);

    // Yank the crate

    let response = token.delete::<()>("/api/v1/crates/serde/1.0.0/yank");
    assert_eq!(response.status(), StatusCode::OK);

    app.run_pending_background_jobs();
    assert_ok_eq!(
        upstream.list_commits(),
        vec![
            "Initial Commit",
            "Create crate `serde`",
            "Update crate `serde`",
        ]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), true);

    // Delete the crate

    app.db(|conn| {
        use crates_io::schema::crates;

        let krate: Crate = assert_ok!(Crate::by_name("serde").first(conn));
        assert_ok!(diesel::delete(crates::table.find(krate.id)).execute(conn));

        assert_ok!(jobs::enqueue_sync_to_index("serde", conn));
    });

    app.run_pending_background_jobs();
    assert_ok_eq!(
        upstream.list_commits(),
        vec![
            "Initial Commit",
            "Create crate `serde`",
            "Update crate `serde`",
            "Delete crate `serde`",
        ]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), false);
}
