use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_github::{GitHubOrganization, GitHubTeam, GitHubTeamMembership, MockGitHubClient};
use http::StatusCode;
use insta::assert_snapshot;

/// See <https://github.com/rust-lang/crates.io/issues/1205>.
#[tokio::test(flavor = "multi_thread")]
async fn test_issue_1205() -> anyhow::Result<()> {
    const CRATE_NAME: &str = "deepspeech-sys";

    let (app, _, _, user) = TestApp::full().with_github(github_mock()).with_token();

    let mut conn = app.db_conn();

    let krate = CrateBuilder::new(CRATE_NAME, user.as_model().id).expect_build(&mut conn);

    let response = user
        .add_named_owner(CRATE_NAME, "github:rustaudio:owners")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"team github:rustaudio:owners has been added as an owner of crate deepspeech-sys","ok":true}"#);

    let owners = krate.owners(&mut conn)?;
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].login(), "foo");
    assert_eq!(owners[1].login(), "github:rustaudio:owners");

    let response = user
        .add_named_owner(CRATE_NAME, "github:rustaudio:cratesio-push")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"team github:rustaudio:cratesio-push has been added as an owner of crate deepspeech-sys","ok":true}"#);

    let owners = krate.owners(&mut conn)?;
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].login(), "foo");
    assert_eq!(owners[1].login(), "github:rustaudio:cratesio-push");

    let response = user
        .remove_named_owner(CRATE_NAME, "github:rustaudio:owners")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find owner with login `github:rustaudio:owners`"}]}"#);

    Ok(())
}

fn github_mock() -> MockGitHubClient {
    use mockall::predicate::*;

    let mut github_mock = MockGitHubClient::new();

    github_mock
        .expect_org_by_name()
        .with(eq("rustaudio"), always())
        .returning(|_, _| Ok(org()));

    github_mock
        .expect_team_by_name()
        .with(eq("rustaudio"), eq("owners"), always())
        .returning(|_, name, _| Ok(team(name)));

    github_mock
        .expect_team_by_name()
        .with(eq("rustaudio"), eq("cratesio-push"), always())
        .returning(|_, name, _| Ok(team(name)));

    github_mock
        .expect_team_membership()
        .with(eq(1), eq(2), eq("foo"), always())
        .returning(|_, _, _, _| Ok(active_membership()));

    github_mock
}

fn org() -> GitHubOrganization {
    GitHubOrganization {
        id: 1,
        avatar_url: None,
    }
}

fn team(name: &str) -> GitHubTeam {
    GitHubTeam {
        id: 2,
        name: Some(name.to_string()),
        organization: org(),
    }
}

fn active_membership() -> GitHubTeamMembership {
    let state = "active".to_string();
    GitHubTeamMembership { state }
}
