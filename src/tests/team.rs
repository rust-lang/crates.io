use crate::models::{Crate, CrateOwner, NewTeam};
use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::{OwnerTeamsResponse, RequestHelper, TestApp, add_team_to_crate, new_team};

use diesel::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::assert_snapshot;

impl crate::tests::util::MockAnonymousUser {
    /// List the team owners of the specified crate.
    async fn crate_owner_teams(
        &self,
        krate_name: &str,
    ) -> crate::tests::util::Response<OwnerTeamsResponse> {
        let url = format!("/api/v1/crates/{krate_name}/owner_team");
        self.get(&url).await
    }
}

/// Test adding team without `github:`
#[tokio::test(flavor = "multi_thread")]
async fn not_github() {
    let (app, _, user, token) = TestApp::init().with_token().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_not_github", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = token
        .add_named_owner("foo_not_github", "dropbox:foo:foo")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown organization handler, only 'github:org:team' is supported"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn weird_name() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_weird_name", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = token
        .add_named_owner("foo_weird_name", "github:foo/../bar:wut")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"organization cannot contain special characters like /"}]}"#);
}

/// Test adding team without second `:`
#[tokio::test(flavor = "multi_thread")]
async fn one_colon() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_one_colon", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner("foo_one_colon", "github:foo").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing github team argument; format is github:org:team"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn add_nonexistent_team() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_add_nonexistent", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = token
        .add_named_owner("foo_add_nonexistent", "github:test-org:this-does-not-exist")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find the github team test-org/this-does-not-exist. Make sure that you have the right permissions in GitHub. See https://doc.rust-lang.org/cargo/reference/publishing.html#github-permissions"}]}"#);
}

/// Test adding a renamed team
#[tokio::test(flavor = "multi_thread")]
async fn add_renamed_team() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("user-all-teams").await;
    let token = user.db_new_token("arbitrary token name").await;
    let owner_id = user.as_model().id;

    use crate::schema::teams;

    CrateBuilder::new("foo_renamed_team", owner_id)
        .expect_build(&mut conn)
        .await;

    // create team with same ID and different name compared to http mock
    // used for `async_add_named_owner`.await
    let new_team = NewTeam::builder()
        // different team name
        .login("github:test-org:old-core")
        // same org ID
        .org_id(1000)
        // same team id as `core` team
        .github_id(2001)
        .build();

    new_team.create_or_update(&mut conn).await?;

    assert_eq!(teams::table.count().get_result::<i64>(&mut conn).await?, 1);

    token
        .add_named_owner("foo_renamed_team", "github:test-org:core")
        .await
        .good();

    let json = anon.crate_owner_teams("foo_renamed_team").await.good();
    assert_eq!(json.teams.len(), 1);
    assert_eq!(json.teams[0].login, "github:test-org:core");

    Ok(())
}

/// Test adding team names with mixed case, when on the team
#[tokio::test(flavor = "multi_thread")]
async fn add_team_mixed_case() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("user-all-teams").await;
    let token = user.db_new_token("arbitrary token name").await;

    CrateBuilder::new("foo_mixed_case", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    token
        .add_named_owner("foo_mixed_case", "github:Test-Org:Core")
        .await
        .good();

    let krate: Crate = Crate::by_name("foo_mixed_case").first(&mut conn).await?;
    let owners = krate.owners(&mut conn).await?;
    assert_eq!(owners.len(), 2);
    let owner = &owners[1];
    assert_eq!(owner.login(), owner.login().to_lowercase());

    let json = anon.crate_owner_teams("foo_mixed_case").await.good();
    assert_eq!(json.teams.len(), 1);
    assert_eq!(json.teams[0].login, "github:test-org:core");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn add_team_as_org_owner() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("user-org-owner").await;
    let token = user.db_new_token("arbitrary token name").await;

    CrateBuilder::new("foo_org_owner", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    token
        .add_named_owner("foo_org_owner", "github:test-org:core")
        .await
        .good();

    let krate: Crate = Crate::by_name("foo_org_owner").first(&mut conn).await?;
    let owners = krate.owners(&mut conn).await?;
    assert_eq!(owners.len(), 2);
    let owner = &owners[1];
    assert_eq!(owner.login(), owner.login().to_lowercase());

    let json = anon.crate_owner_teams("foo_org_owner").await.good();
    assert_eq!(json.teams.len(), 1);
    assert_eq!(json.teams[0].login, "github:test-org:core");

    Ok(())
}

/// Test adding team as owner when not on it
#[tokio::test(flavor = "multi_thread")]
async fn add_team_as_non_member() {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("user-one-team").await;
    let token = user.db_new_token("arbitrary token name").await;

    CrateBuilder::new("foo_team_non_member", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = token
        .add_named_owner("foo_team_non_member", "github:test-org:core")
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only members of a team or organization owners can add it as an owner"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_team_as_named_owner() {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;
    let username = "user-all-teams";
    let user_on_both_teams = app.db_new_user(username).await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_remove_team", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_remove_team", "github:test-org:core")
        .await
        .good();

    // Removing the individual owner is not allowed, since team members don't
    // have permission to manage ownership
    let response = token_on_both_teams
        .remove_named_owner("foo_remove_team", username)
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required."}]}"#);

    token_on_both_teams
        .remove_named_owner("foo_remove_team", "github:test-org:core")
        .await
        .good();

    let user_on_one_team = app.db_new_user("user-one-team").await;
    let crate_to_publish = PublishBuilder::new("foo_remove_team", "2.0.0");
    let response = user_on_one_team.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_team_as_team_owner() {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_remove_team_owner", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_remove_team_owner", "github:test-org:all")
        .await
        .good();

    let user_on_one_team = app.db_new_user("user-one-team").await;
    let token_on_one_team = user_on_one_team.db_new_token("arbitrary token name").await;

    let response = token_on_one_team
        .remove_named_owner("foo_remove_team_owner", "github:test-org:all")
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"team members don't have permission to modify owners"}]}"#);

    let user_org_owner = app.db_new_user("user-org-owner").await;
    let token_org_owner = user_org_owner.db_new_token("arbitrary token name").await;
    let response = token_org_owner
        .remove_named_owner("foo_remove_team_owner", "github:test-org:all")
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only owners have permission to modify owners"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_nonexistent_team() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    let krate = CrateBuilder::new("foo_remove_nonexistent", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let team = NewTeam::builder()
        .login("github:test-org:this-does-not-exist")
        .github_id(5678)
        .org_id(1234)
        .build()
        .create_or_update(&mut conn)
        .await
        .expect("couldn't insert nonexistent team");

    CrateOwner::builder()
        .crate_id(krate.id)
        .team_id(team.id)
        .created_by(user.as_model().id)
        .build()
        .insert(&mut conn)
        .await
        .unwrap();

    let response = token
        .remove_named_owner(
            "foo_remove_nonexistent",
            "github:test-org:this-does-not-exist",
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
}

/// Test trying to publish a crate we don't own
#[tokio::test(flavor = "multi_thread")]
async fn publish_not_owned() {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_not_owned", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_not_owned", "github:test-org:core")
        .await
        .good();

    let user_on_one_team = app.db_new_user("user-one-team").await;

    let crate_to_publish = PublishBuilder::new("foo_not_owned", "2.0.0");
    let response = user_on_one_team.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_org_owner_owned() {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_not_owned", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_not_owned", "github:test-org:core")
        .await
        .good();

    let user_org_owner = app.db_new_user("user-org-owner").await;

    let crate_to_publish = PublishBuilder::new("foo_not_owned", "2.0.0");
    let response = user_org_owner.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing."}]}"#);
}

/// Test trying to publish a krate we do own (but only because of teams)
#[tokio::test(flavor = "multi_thread")]
async fn publish_owned() {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_team_owned", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_team_owned", "github:test-org:all")
        .await
        .good();

    let user_on_one_team = app.db_new_user("user-one-team").await;

    let crate_to_publish = PublishBuilder::new("foo_team_owned", "2.0.0");
    user_on_one_team
        .publish_crate(crate_to_publish)
        .await
        .good();

    assert_snapshot!(app.emails_snapshot().await);
}

/// Test trying to change owners (when only on an owning team)
#[tokio::test(flavor = "multi_thread")]
async fn add_owners_as_org_owner() {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_add_owner", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_add_owner", "github:test-org:all")
        .await
        .good();

    let user_org_owner = app.db_new_user("user-org-owner").await;
    let token_org_owner = user_org_owner.db_new_token("arbitrary token name").await;

    let response = token_org_owner
        .add_named_owner("foo_add_owner", "arbitrary_username")
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only owners have permission to modify owners"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn add_owners_as_team_owner() {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user_on_both_teams = app.db_new_user("user-all-teams").await;
    let token_on_both_teams = user_on_both_teams
        .db_new_token("arbitrary token name")
        .await;

    CrateBuilder::new("foo_add_owner", user_on_both_teams.as_model().id)
        .expect_build(&mut conn)
        .await;

    token_on_both_teams
        .add_named_owner("foo_add_owner", "github:test-org:all")
        .await
        .good();

    let user_on_one_team = app.db_new_user("user-one-team").await;
    let token_on_one_team = user_on_one_team.db_new_token("arbitrary token name").await;

    let response = token_on_one_team
        .add_named_owner("foo_add_owner", "arbitrary_username")
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"team members don't have permission to modify owners"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_by_team_id() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let t = new_team("github:test-org:team")
        .create_or_update(&mut conn)
        .await?;
    let krate = CrateBuilder::new("foo", user.id)
        .expect_build(&mut conn)
        .await;
    add_team_to_crate(&t, &krate, user, &mut conn).await?;

    let json = anon.search(&format!("team_id={}", t.id)).await;
    assert_eq!(json.crates.len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_by_team_id_not_including_deleted_owners() -> anyhow::Result<()> {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("user-all-teams").await;
    let user = user.as_model();

    let new_team = NewTeam::builder()
        .login("github:test-org:core")
        .org_id(1000)
        .github_id(2001)
        .build();

    let t = new_team.create_or_update(&mut conn).await?;

    let krate = CrateBuilder::new("foo", user.id)
        .expect_build(&mut conn)
        .await;
    add_team_to_crate(&t, &krate, user, &mut conn).await?;
    krate.owner_remove(&mut conn, &t.login).await.unwrap();

    let json = anon.search(&format!("team_id={}", t.id)).await;
    assert_eq!(json.crates.len(), 0);

    Ok(())
}
