use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crate::{add_team_to_crate, new_team};
use crates_io_github::{GitHubError, MockGitHubClient};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn publish_with_org_restrictions() {
    let mut github = MockGitHubClient::new();

    // Standard expectations for login/token creation
    github.expect_current_user().returning(|_| {
        Ok(crates_io_github::GitHubUser {
            id: 1,
            login: "foo".into(),
            name: Some("Foo".into()),
            email: Some("foo@example.com".into()),
            avatar_url: None,
        })
    });

    // Mock 403 Forbidden for team membership check
    github
        .expect_team_membership()
        .returning(|_org_id, _team_id, _username, _token| {
            Err(GitHubError::Forbidden(anyhow::anyhow!("403 Forbidden")))
        });

    let (app, _, owner, _owner_token) = TestApp::full().with_github(github).with_token().await;
    let mut conn = app.db_conn().await;
    let owner_model = owner.as_model();

    // Create a second user who will try to publish
    let user = app.db_new_user("bar").await;
    let token = user.db_new_token("bar_token").await;

    // Set up a crate owned by the first user
    let krate = CrateBuilder::new("foo_crate", owner_model.id)
        .expect_build(&mut conn)
        .await;

    // Add the team as an owner
    let team = new_team("github:servo:graphics")
        .create_or_update(&conn)
        .await
        .unwrap();
    add_team_to_crate(&team, &krate, owner_model, &mut conn)
        .await
        .unwrap();

    // Now try to publish as 'user' (bar)
    let crate_to_publish = PublishBuilder::new("foo_crate", "2.0.0");
    let response = token.publish_crate(crate_to_publish).await;

    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"GitHub organization 'servo' has restricted OAuth access. A 'servo' administrator must approve the 'crates.io' application in the organization's 'Third-party access' settings."}]}"#);
}
