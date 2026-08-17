use crate::builders::{CrateBuilder, OauthGithubBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::models::CrateOwner;
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

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_ambiguous_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice", "alice-gh").await;
    let github_alice = app.db_new_user_with_gh_login("bob", "alice").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for user in [&cratesio_alice, &github_alice] {
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let response = cookie.remove_named_owner("foo", "alice").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"username `alice` is ambiguous. There are two owners of this crate with the username `alice` on different services.\n\nTo confirm which owner you want to remove, please run one of the following:\n\n$ cargo owner --remove crates.io:alice\n$ cargo owner --remove github:alice\n\nIf this is not the account you want to remove, verify the crates.io username of the account you want."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_ambiguous_user_with_cratesio_prefix() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice", "alice-gh").await;
    let github_alice = app.db_new_user_with_gh_login("bob", "alice").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for user in [&cratesio_alice, &github_alice] {
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let response = cookie.remove_named_owner("foo", "crates.io:alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_ambiguous_user_with_github_prefix() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice", "alice-gh").await;
    let github_alice = app.db_new_user_with_gh_login("bob", "alice").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for user in [&cratesio_alice, &github_alice] {
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let response = cookie.remove_named_owner("foo", "github:alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_ambiguous_user_differing_only_by_separator() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice-2", "alice-2-gh").await;
    OauthGithubBuilder::for_user(cratesio_alice.as_model())
        .with_login("alice-2-gh")
        .insert(&conn)
        .await;

    let github_alice = app.db_new_user_with_gh_login("bob", "alice_2").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice_2")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    for user in [&cratesio_alice, &github_alice] {
        CrateOwner::builder()
            .crate_id(krate.id)
            .user_id(user.as_model().id)
            .created_by(cookie.as_model().id)
            .build()
            .insert(&conn)
            .await
            .unwrap();
    }

    let response = cookie.remove_named_owner("foo", "alice-2").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"username `alice-2` is ambiguous. There are two owners of this crate with the username `alice-2` on different services.\n\nTo confirm which owner you want to remove, please run one of the following:\n\n$ cargo owner --remove crates.io:alice-2\n$ cargo owner --remove github:alice-2\n\nIf this is not the account you want to remove, verify the crates.io username of the account you want."}]}"#);

    // The suggested commands name the owners as they are stored, so both work.
    let response = cookie.remove_named_owner("foo", "github:alice_2").await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = cookie.remove_named_owner("foo", "crates.io:alice-2").await;
    assert_snapshot!(response.status(), @"200 OK");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_shared_login_when_only_cratesio_user_is_owner() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice", "alice-gh").await;
    OauthGithubBuilder::for_user(cratesio_alice.as_model())
        .with_login("alice-gh")
        .insert(&conn)
        .await;

    let github_alice = app.db_new_user_with_gh_login("bob", "alice").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(cratesio_alice.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.remove_named_owner("foo", "alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_shared_login_when_only_github_user_is_owner() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let cratesio_alice = app.db_new_user_with_gh_login("alice", "alice-gh").await;
    OauthGithubBuilder::for_user(cratesio_alice.as_model())
        .with_login("alice-gh")
        .insert(&conn)
        .await;

    let github_alice = app.db_new_user_with_gh_login("bob", "alice").await;
    OauthGithubBuilder::for_user(github_alice.as_model())
        .with_login("alice")
        .insert(&conn)
        .await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(github_alice.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.remove_named_owner("foo", "alice").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_unprefixed_non_ambiguous() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("user2").await;
    let mut conn = app.db_conn().await;

    OauthGithubBuilder::for_user(user2.as_model())
        .with_login("user2")
        .insert(&conn)
        .await;

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

    let response = cookie.remove_named_owner("foo", "user2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_unprefixed_username_only() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user_with_gh_login("user2", "user2-gh").await;
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

    let response = cookie.remove_named_owner("foo", "user2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_mixed_case_cratesio() {
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

    let response = cookie.remove_named_owner("foo", "crates.io:USer2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_mixed_case_github() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user_with_gh_login("user2", "user2-gh").await;
    let mut conn = app.db_conn().await;

    OauthGithubBuilder::for_user(user2.as_model())
        .with_login("user2-gh")
        .insert(&conn)
        .await;

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

    let response = cookie.remove_named_owner("foo", "github:useR2-gH").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_separator_variant_unprefixed() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("user-2").await;
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

    let response = cookie.remove_named_owner("foo", "user_2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_separator_variant_cratesio() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user("user-2").await;
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

    let response = cookie.remove_named_owner("foo", "crates.io:user_2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_remove_separator_variant_github() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user_with_gh_login("user2", "user-2-gh").await;
    let mut conn = app.db_conn().await;

    OauthGithubBuilder::for_user(user2.as_model())
        .with_login("user-2-gh")
        .insert(&conn)
        .await;

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

    let response = cookie.remove_named_owner("foo", "github:user_2_gh").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_team_with_extra_component() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie
        .remove_named_owner("foo", "github:alice:team:extra")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid argument. only github:org:team, github:username, crates.io:username and username are supported."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_empty_org() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "github::team").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"organization cannot be empty"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_empty_team() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "github:org:").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"team cannot be empty"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_empty_cratesio_username() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "crates.io:").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"username cannot be empty"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_empty_login() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"username cannot be empty"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reject_github_username_with_invalid_char() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "github:a&lice*").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"username cannot contain special characters like &"}]}"#);
}

/// Test that an unsupported prefix (e.g. gitlab:) returns an error.
#[tokio::test(flavor = "multi_thread")]
async fn test_unsupported_disambiguation_prefix() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "gitlab:user2").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid argument. only github:org:team, github:username, crates.io:username and username are supported."}]}"#);
}

/// Test that removing with nonexistent github username returns an error.
#[tokio::test(flavor = "multi_thread")]
async fn test_disambiguated_github_username_not_found() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.remove_named_owner("foo", "github:nonexistent").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `nonexistent`"}]}"#);
}

/// Test that removing with nonexistent crates.io username returns an error.
#[tokio::test(flavor = "multi_thread")]
async fn test_disambiguated_cratesio_username_not_found() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie
        .remove_named_owner("foo", "crates.io:nonexistent")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `nonexistent`"}]}"#);
}

/// Test that removing an ambiguous user with github: prefix works.
#[tokio::test(flavor = "multi_thread")]
async fn test_disambiguate_remove_with_github_prefix() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user_with_gh_login("user2", "user2-gh").await;
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

    OauthGithubBuilder::for_user(user2.as_model())
        .with_login("user2-gh")
        .insert(&conn)
        .await;

    let response = cookie.remove_named_owner("foo", "github:user2-gh").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

/// Test that removing an ambiguous user with crates.io: prefix works .
#[tokio::test(flavor = "multi_thread")]
async fn test_disambiguate_remove_with_cratesio_prefix() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let user2 = app.db_new_user_with_gh_login("user2", "user2-gh").await;
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

    let response = cookie.remove_named_owner("foo", "crates.io:user2").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}
