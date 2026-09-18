use crate::OwnerResp;
use crate::builders::{CrateBuilder, UserBuilder};
use crate::util::{RequestHelper, Response, TestApp};
use crates_io::models::{CrateOwner, User};
use crates_io_github::{GitHubOrganization, GitHubTeam, GitHubTeamMembership, MockGitHubClient};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_invalid_json() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    app.db_new_user("bar").await;
    CrateBuilder::new("foo", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    // incomplete input
    let input = r#"{"owners": ["foo", }"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: owners[1]: expected value at line 1 column 20"}]}"#);

    // `owners` is not an array
    let input = r#"{"owners": "foo"}"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: owners: invalid type: string \"foo\", expected a sequence at line 1 column 16"}]}"#);

    // missing `owners` and/or `users` fields
    let input = r#"{}"#;
    let response = user
        .delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `owners` at line 1 column 2"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user().await;
    app.db_new_user("bar").await;

    let response = user.remove_named_owner("unknown", "bar").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "unknown").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_team() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie
        .remove_named_owner("foo", "github:unknown:unknown")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:unknown:unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_uppercase_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("user2").await;
    let mut conn = app.db_conn().await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(user2.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.remove_named_owner("foo", "USER2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

/// One GitHub login can remove multiple owners while leaving another user owner.
#[tokio::test(flavor = "multi_thread")]
async fn reused_github_login_removes_all_matching_owners() {
    remove_reused_github_login("shared").await;
}

/// A GitHub prefix retains removal of all matching owners.
#[tokio::test(flavor = "multi_thread")]
async fn prefixed_reused_github_login_removes_all_matching_owners() {
    remove_reused_github_login("github:shared").await;
}

/// Checks that a reused login removes every matching owner.
async fn remove_reused_github_login(login: &str) {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for (username, gh_login) in [("older", "shared"), ("newer", "SHARED")] {
        let user = UserBuilder::new()
            .with_username(username)
            .with_gh_login(gh_login);
        let user = app.db_new_user_from_builder(user).await;
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let response = cookie.remove_named_owner("foo", login).await;
    assert_eq!(response.status(), http::StatusCode::OK);

    let owners = User::owning(&krate, &conn).await.unwrap();
    let owner_ids: Vec<_> = owners.iter().map(|owner| owner.id).collect();
    assert_eq!(owner_ids, [cookie.as_model().id]);
}

/// Different namespace selections reject the whole batch, even with an overlapping owner.
#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_removal_with_conflicting_owners() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let alice = app.db_new_user("alice").await;
    let bob = UserBuilder::new()
        .with_username("bob")
        .with_gh_login("alice");
    let bob = app.db_new_user_from_builder(bob).await;
    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for user in [&alice, &bob] {
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let logins = ["crates.io:alice", "ALICE"];
    let response = cookie.remove_named_owners("foo", &logins).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The username `ALICE` matches different owners. Use `crates.io:ALICE` or `github:ALICE` to select which owner to remove."}]}"#);
    assert_eq!(krate.owners(&conn).await.unwrap().len(), 3);

    let response = cookie.remove_named_owner("foo", "crates.io:alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(krate.owners(&conn).await.unwrap().len(), 2);

    let response = cookie.remove_named_owner("foo", "github:alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(krate.owners(&conn).await.unwrap().len(), 1);
}

async fn remove_distinct_login_user(login: &str) -> (Response<OwnerResp>, usize) {
    remove_distinct_login_users(&[login]).await
}

/// Removes owners with distinct names, ignoring a non-owner with opposing names.
async fn remove_distinct_login_users(logins: &[&str]) -> (Response<OwnerResp>, usize) {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user2 = UserBuilder::new()
        .with_username("crates-user")
        .with_gh_login("github-user");
    let user2 = app.db_new_user_from_builder(user2).await;
    let non_owner = UserBuilder::new()
        .with_username("github-user")
        .with_gh_login("crates-user");
    app.db_new_user_from_builder(non_owner).await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(user2.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.remove_named_owners(&krate.name, logins).await;
    let owner_count = krate.owners(&conn).await.unwrap().len();
    (response, owner_count)
}

/// Both namespaces select the same owner from the initial snapshot.
#[tokio::test(flavor = "multi_thread")]
async fn overlapping_prefixed_removals() {
    let logins = [
        "crates.io:CRATES_USER",
        "github:github-user",
        "github:github-user",
    ];
    let (response, owner_count) = remove_distinct_login_users(&logins).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(owner_count, 1);
}

/// A missing prefixed owner prevents the complete batch from being removed.
#[tokio::test(flavor = "multi_thread")]
async fn prefixed_removal_missing_owner() {
    let logins = ["crates.io:crates-user", "github:missing"];
    let (response, owner_count) = remove_distinct_login_users(&logins).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:missing`"}]}"#);
    assert_eq!(owner_count, 2);
}

/// Removing the last individual owners rolls back the batch.
#[tokio::test(flavor = "multi_thread")]
async fn prefixed_removal_of_all_individual_owners() {
    let logins = ["crates.io:foo", "github:github-user"];
    let (response, owner_count) = remove_distinct_login_users(&logins).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required."}]}"#);
    assert_eq!(owner_count, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_verbatim() {
    let (response, owner_count) = remove_distinct_login_user("crates-user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_case_insensitive() {
    let (response, owner_count) = remove_distinct_login_user("CRATES-USER").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_separator_variant() {
    let (response, owner_count) = remove_distinct_login_user("crates_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_verbatim() {
    let (response, owner_count) = remove_distinct_login_user("github-user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_case_insensitive() {
    let (response, owner_count) = remove_distinct_login_user("GITHUB-USER").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_separator_variant() {
    let (response, owner_count) = remove_distinct_login_user("github_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github_user`"}]}"#);
    assert_eq!(owner_count, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_verbatim() {
    let (response, owner_count) = remove_distinct_login_user("crates.io:crates-user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_case_insensitive() {
    let (response, owner_count) = remove_distinct_login_user("crates.io:CRATES-USER").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_separator_variant() {
    let (response, owner_count) = remove_distinct_login_user("crates.io:crates_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_verbatim() {
    let (response, owner_count) = remove_distinct_login_user("github:github-user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_case_insensitive() {
    let (response, owner_count) = remove_distinct_login_user("github:GITHUB-USER").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(owner_count, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_separator_variant() {
    let (response, owner_count) = remove_distinct_login_user("github:github_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:github_user`"}]}"#);
    assert_eq!(owner_count, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefix_does_not_match_github_login() {
    let (response, owner_count) = remove_distinct_login_user("crates.io:github-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `crates.io:github-user`"}]}"#);
    assert_eq!(owner_count, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefix_does_not_match_crates_io_username() {
    let (response, owner_count) = remove_distinct_login_user("github:crates-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:crates-user`"}]}"#);
    assert_eq!(owner_count, 2);
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
            Ok(Some(GitHubTeamMembership {
                state: "active".to_string(),
            }))
        });

    let (app, _, cookie) = TestApp::full().with_github(github_mock).with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("crate42", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.add_named_owner("crate42", "github:org:team").await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = cookie
        .remove_named_owner("crate42", "github:ORG:TEAM")
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}
