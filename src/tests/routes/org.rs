use crate::util::{RequestHelper, TestApp};
use crates_io::models::NewTeam;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn teams_by_org() {
    let (app, anon) = TestApp::init().empty().await;
    let conn = app.db_conn().await;

    // Two teams in this org that should be returned
    NewTeam::builder()
        .login("github:rust-lang:releasers")
        .github_id(5678)
        .org_id(1234)
        .build()
        .create_or_update(&conn)
        .await
        .unwrap();

    NewTeam::builder()
        .login("github:rust-lang:libs")
        .github_id(5679)
        .org_id(1234)
        .build()
        .create_or_update(&conn)
        .await
        .unwrap();

    // One team not in this org that should not be returned
    NewTeam::builder()
        .login("github:test-org:test-team")
        .github_id(3333)
        .org_id(1235)
        .build()
        .create_or_update(&conn)
        .await
        .unwrap();

    let response = anon.get::<()>("/api/v1/orgs/github:rust-lang").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    // Case insensitive
    let response = anon.get::<()>("/api/v1/orgs/gItHuB:tEsT-oRg").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    // Unknown org
    let response = anon
        .get::<()>("/api/v1/orgs/github:not-an-org-we-know-about")
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    // Wildcards in the org name are invalid
    let response = anon
        .get::<()>("/api/v1/orgs/github:%")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    let response = anon
        .get::<()>("/api/v1/orgs/git__b:_________")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // No colons in the org name
    let response = anon.get::<()>("/api/v1/orgs/rust-lang").await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Too many colons in the org name
    let response = anon.get::<()>("/api/v1/orgs/github:rust-lang:libs").await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Nothing before the colon
    let response = anon.get::<()>("/api/v1/orgs/:rust-lang").await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    // Nothing after the colon
    let response = anon.get::<()>("/api/v1/orgs/github:").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
}
