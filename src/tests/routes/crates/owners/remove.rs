use crate::models::{CrateOwner, OwnerKind};
use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::schema::crate_owners;
use crates_io_github::{GitHubOrganization, GitHubTeam, GitHubTeamMembership, MockGitHubClient};
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_invalid_json() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.async_db_conn().await;

    app.db_new_user("bar").await;
    CrateBuilder::new("foo", user.as_model().id)
        .async_expect_build(&mut conn)
        .await;

    // incomplete input
    let input = r#"{"owners": ["foo", }"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: owners[1]: expected value at line 1 column 20"}]}"#);

    // `owners` is not an array
    let input = r#"{"owners": "foo"}"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: owners: invalid type: string \"foo\", expected a sequence at line 1 column 16"}]}"#);

    // missing `owners` and/or `users` fields
    let input = r#"{}"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `owners` at line 1 column 2"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user().await;
    app.db_new_user("bar").await;

    let response = user.remove_named_owner("unknown", "bar").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.async_db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .async_expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "unknown").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_team() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.async_db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .async_expect_build(&mut conn)
        .await;

    let response = cookie
        .remove_named_owner("foo", "github:unknown:unknown")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:unknown:unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_uppercase_user() {
    use diesel::RunQueryDsl;

    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("user2").await;
    let mut conn = app.db_conn();
    let mut async_conn = app.async_db_conn().await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .async_expect_build(&mut async_conn)
        .await;

    diesel::insert_into(crate_owners::table)
        .values(CrateOwner {
            crate_id: krate.id,
            owner_id: user2.as_model().id,
            created_by: cookie.as_model().id,
            owner_kind: OwnerKind::User,
            email_notifications: true,
        })
        .execute(&mut conn)
        .unwrap();

    let response = cookie.remove_named_owner("foo", "USER2").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_uppercase_team() {
    use mockall::predicate::*;

    let mut github_mock = MockGitHubClient::new();

    github_mock
        .expect_team_by_name()
        .with(eq("org"), eq("team"), always())
        .returning(|_, _, _| {
            Ok(GitHubTeam {
                id: 2,
                name: Some("team".to_string()),
                organization: GitHubOrganization {
                    id: 1,
                    avatar_url: None,
                },
            })
        });

    github_mock
        .expect_org_by_name()
        .with(eq("org"), always())
        .returning(|_, _| {
            Ok(GitHubOrganization {
                id: 1,
                avatar_url: None,
            })
        });

    github_mock
        .expect_team_membership()
        .with(eq(1), eq(2), eq("foo"), always())
        .returning(|_, _, _, _| {
            Ok(GitHubTeamMembership {
                state: "active".to_string(),
            })
        });

    let (app, _, cookie) = TestApp::full().with_github(github_mock).with_user().await;
    let mut conn = app.async_db_conn().await;

    CrateBuilder::new("crate42", cookie.as_model().id)
        .async_expect_build(&mut conn)
        .await;

    let response = cookie.add_named_owner("crate42", "github:org:team").await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = cookie
        .remove_named_owner("crate42", "github:ORG:TEAM")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}
