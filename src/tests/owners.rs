use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{MockAnonymousUser, MockCookieUser, MockTokenUser, RequestHelper, Response};
use crate::{TestApp, add_team_to_crate, new_team};
use crates_io::models::Crate;
use crates_io::schema::emails;
use crates_io::views::{
    EncodableCrateOwnerInvitationV1, EncodableOwner, EncodablePublicUser, InvitationResponse,
};

use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::assert_snapshot;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct TeamResponse {
    teams: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
struct UserResponse {
    users: Vec<EncodableOwner>,
}
#[derive(Deserialize, Debug, PartialEq, Eq)]
struct InvitationListResponse {
    crate_owner_invitations: Vec<EncodableCrateOwnerInvitationV1>,
    users: Vec<EncodablePublicUser>,
}

// Implementing locally for now, unless these are needed elsewhere
impl MockCookieUser {
    async fn try_accept_ownership_invitation<T: serde::de::DeserializeOwned>(
        &self,
        krate_name: &str,
        krate_id: i32,
    ) -> Response<T> {
        let body = json!({
            "crate_owner_invite": {
                "invited_by_username": "",
                "crate_name": krate_name,
                "crate_id": krate_id,
                "created_at": "",
                "accepted": true
            }
        });

        let url = format!("/api/v1/me/crate_owner_invitations/{krate_id}");
        self.put(&url, body.to_string()).await
    }

    /// As the currently logged in user, accept an invitation to become an owner of the named
    /// crate.
    async fn accept_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
        #[derive(Deserialize)]
        struct CrateOwnerInvitation {
            crate_owner_invitation: InvitationResponse,
        }

        let crate_owner_invite: CrateOwnerInvitation = self
            .try_accept_ownership_invitation(krate_name, krate_id)
            .await
            .good();

        assert!(crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// As the currently logged in user, decline an invitation to become an owner of the named
    /// crate.
    async fn decline_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
        let body = json!({
            "crate_owner_invite": {
                "invited_by_username": "",
                "crate_name": krate_name,
                "crate_id": krate_id,
                "created_at": "",
                "accepted": false
            }
        });

        #[derive(Deserialize)]
        struct CrateOwnerInvitation {
            crate_owner_invitation: InvitationResponse,
        }

        let url = format!("/api/v1/me/crate_owner_invitations/{krate_id}");
        let crate_owner_invite: CrateOwnerInvitation =
            self.put(&url, body.to_string()).await.good();
        assert!(!crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// As the currently logged in user, list my pending invitations.
    async fn list_invitations(&self) -> InvitationListResponse {
        self.get("/api/v1/me/crate_owner_invitations").await.good()
    }
}

impl MockAnonymousUser {
    async fn accept_ownership_invitation_by_token(&self, token: &str) {
        #[derive(Deserialize)]
        struct Response {
            crate_owner_invitation: InvitationResponse,
        }

        let response: Response = self
            .try_accept_ownership_invitation_by_token(token)
            .await
            .good();
        assert!(response.crate_owner_invitation.accepted);
    }

    async fn try_accept_ownership_invitation_by_token<T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
    ) -> Response<T> {
        let url = format!("/api/v1/me/crate_owner_invitations/accept/{token}");
        self.put(&url, &[] as &[u8]).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_owner() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    // Create a crate under one user
    let crate_to_publish = PublishBuilder::new("foo_owner", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    // Add the second user as an owner (with a different case to make sure that works)
    let user2 = app.db_new_user("Bar").await;
    token.add_named_owner("foo_owner", "BAR").await.good();

    assert_snapshot!(app.emails_snapshot().await);

    // accept invitation for user to be added as owner
    let krate: Crate = Crate::by_name("foo_owner").first(&mut conn).await.unwrap();
    user2
        .accept_ownership_invitation("foo_owner", krate.id)
        .await;

    // Make sure this shows up as one of their crates.
    let crates = user2
        .search(&format!("user_id={}", user2.as_model().id))
        .await;
    assert_eq!(crates.crates.len(), 1);

    // And upload a new version as the second user
    let crate_to_publish = PublishBuilder::new("foo_owner", "2.0.0");
    user2
        .db_new_token("bar_token")
        .await
        .publish_crate(crate_to_publish)
        .await
        .good();

    assert_snapshot!(app.emails_snapshot().await);
}

async fn create_and_add_owner(
    app: &TestApp,
    token: &MockTokenUser,
    username: &str,
    krate: &Crate,
) -> MockCookieUser {
    let user = app.db_new_user(username).await;
    token.add_named_owner(&krate.name, username).await.good();
    user.accept_ownership_invitation(&krate.name, krate.id)
        .await;
    user
}

/// Ensures that so long as at least one owner remains associated with the crate,
/// a user can still remove their own login as an owner
#[tokio::test(flavor = "multi_thread")]
async fn owners_can_remove_self() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let username = &user.as_model().gh_login;

    let krate = CrateBuilder::new("owners_selfremove", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    // Deleting yourself when you're the only owner isn't allowed.
    let response = token
        .remove_named_owner("owners_selfremove", username)
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required."}]}"#);

    create_and_add_owner(&app, &token, "secondowner", &krate).await;

    // Deleting yourself when there are other owners is allowed.
    let response = token
        .remove_named_owner("owners_selfremove", username)
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);

    // After you delete yourself, you no longer have permissions to manage the crate.
    let response = token
        .remove_named_owner("owners_selfremove", username)
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only owners have permission to modify owners"}]}"#);
}

/// Verify consistency when adidng or removing multiple owners in a single request.
#[tokio::test(flavor = "multi_thread")]
async fn modify_multiple_owners() -> anyhow::Result<()> {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let username = &user.as_model().gh_login;

    let krate = CrateBuilder::new("owners_multiple", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let user2 = create_and_add_owner(&app, &token, "user2", &krate).await;
    let user3 = create_and_add_owner(&app, &token, "user3", &krate).await;

    assert_snapshot!(app.emails_snapshot().await);

    // Deleting all owners is not allowed.
    let response = token
        .remove_named_owners("owners_multiple", &[username, "user2", "user3"])
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required."}]}"#);
    assert_eq!(krate.owners(&mut conn).await?.len(), 3);

    // Deleting two owners at once is allowed.
    let response = token
        .remove_named_owners("owners_multiple", &["user2", "user3"])
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);
    assert_eq!(krate.owners(&mut conn).await?.len(), 1);

    // Adding multiple users fails if one of them already is an owner.
    let response = token
        .add_named_owners("owners_multiple", &["user2", username])
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"`foo` is already an owner"}]}"#);
    assert_eq!(krate.owners(&mut conn).await?.len(), 1);

    // Adding multiple users at once succeeds.
    let response = token
        .add_named_owners("owners_multiple", &["user2", "user3"])
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"user user2 has been invited to be an owner of crate owners_multiple,user user3 has been invited to be an owner of crate owners_multiple","ok":true}"#);

    assert_snapshot!(app.emails_snapshot().await);

    user2
        .accept_ownership_invitation(&krate.name, krate.id)
        .await;
    user3
        .accept_ownership_invitation(&krate.name, krate.id)
        .await;

    assert_eq!(krate.owners(&mut conn).await?.len(), 3);

    Ok(())
}

/// Testing the crate ownership between two crates and one team.
/// Given two crates, one crate owned by both a team and a user,
/// one only owned by a user, check that the CrateList returned
/// for the user_id contains only the crates owned by that user,
/// and that the CrateList returned for the team_id contains
/// only crates owned by that team.
#[tokio::test(flavor = "multi_thread")]
async fn check_ownership_two_crates() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let team = new_team("team_foo").create_or_update(&mut conn).await?;
    let krate_owned_by_team = CrateBuilder::new("foo", user.id)
        .expect_build(&mut conn)
        .await;
    add_team_to_crate(&team, &krate_owned_by_team, user, &mut conn).await?;

    let user2 = app.db_new_user("user_bar").await;
    let user2 = user2.as_model();
    let krate_not_owned_by_team = CrateBuilder::new("bar", user2.id)
        .expect_build(&mut conn)
        .await;

    let json = anon.search(&format!("user_id={}", user2.id)).await;
    assert_eq!(json.crates[0].name, krate_not_owned_by_team.name);
    assert_eq!(json.crates.len(), 1);

    let query = format!("team_id={}", team.id);
    let json = anon.search(&query).await;
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.crates[0].name, krate_owned_by_team.name);

    Ok(())
}

/// Given a crate owned by both a team and a user, check that the
/// JSON returned by the /owner_team route and /owner_user route
/// contains the correct kind of owner
///
/// Note that in this case function new_team must take a team name
/// of form github:org_name:team_name as that is the format
/// EncodableOwner::encodable is expecting
#[tokio::test(flavor = "multi_thread")]
async fn check_ownership_one_crate() -> anyhow::Result<()> {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let team = new_team("github:test_org:team_sloth")
        .create_or_update(&mut conn)
        .await?;
    let krate = CrateBuilder::new("best_crate", user.id)
        .expect_build(&mut conn)
        .await;
    add_team_to_crate(&team, &krate, user, &mut conn).await?;

    let json: TeamResponse = anon
        .get("/api/v1/crates/best_crate/owner_team")
        .await
        .good();
    assert_eq!(json.teams[0].kind, "team");
    assert_eq!(json.teams[0].name, team.name);

    let json: UserResponse = anon
        .get("/api/v1/crates/best_crate/owner_user")
        .await
        .good();
    assert_eq!(json.users[0].kind, "user");
    assert_eq!(json.users[0].name, user.name);

    Ok(())
}

/// Assert the error response when attempting to add a team as a crate owner
/// when that team is already a crate owner.
#[tokio::test(flavor = "multi_thread")]
async fn add_existing_team() {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let t = new_team("github:test_org:bananas")
        .create_or_update(&mut conn)
        .await
        .unwrap();
    let krate = CrateBuilder::new("best_crate", user.id)
        .expect_build(&mut conn)
        .await;
    add_team_to_crate(&t, &krate, user, &mut conn)
        .await
        .unwrap();

    let ret = token
        .add_named_owner("best_crate", "github:test_org:bananas")
        .await;
    assert_eq!(ret.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        ret.text(),
        r#"{"errors":[{"detail":"`github:test_org:bananas` is already an owner"}]}"#
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn deleted_ownership_isnt_in_owner_user() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate = CrateBuilder::new("foo_my_packages", user.id)
        .expect_build(&mut conn)
        .await;
    krate.owner_remove(&mut conn, &user.gh_login).await.unwrap();

    let json: UserResponse = anon
        .get("/api/v1/crates/foo_my_packages/owner_user")
        .await
        .good();
    assert_eq!(json.users.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user().await;
    app.db_new_user("bar").await;

    let response = user.get::<()>("/api/v1/crates/unknown/owners").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);

    let response = user.get::<()>("/api/v1/crates/unknown/owner_team").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);

    let response = user.get::<()>("/api/v1/crates/unknown/owner_user").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_are_empty_by_default_v1() {
    let (_, _, user) = TestApp::init().with_user().await;

    let json = user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn api_token_cannot_list_invitations_v1() {
    let (_, _, _, token) = TestApp::init().with_token().await;

    let response = token.get::<()>("/api/v1/me/crate_owner_invitations").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_v1() {
    let (app, _, owner, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    let krate = CrateBuilder::new("invited_crate", owner.id)
        .expect_build(&mut conn)
        .await;

    let user = app.db_new_user("invited_user").await;
    token
        .add_named_owner("invited_crate", "invited_user")
        .await
        .good();

    let response = user.get::<()>("/api/v1/me/crate_owner_invitations").await;
    assert_snapshot!(response.status(), @"200 OK");

    let invitations = user.list_invitations().await;
    assert_eq!(
        invitations,
        InvitationListResponse {
            crate_owner_invitations: vec![EncodableCrateOwnerInvitationV1 {
                crate_id: krate.id,
                crate_name: krate.name,
                invited_by_username: owner.gh_login.clone(),
                invitee_id: user.as_model().id,
                inviter_id: owner.id,
                // This value changes with each test run so we can't use a fixed value here
                created_at: invitations.crate_owner_invitations[0].created_at,
                // This value changes with each test run so we can't use a fixed value here
                expires_at: invitations.crate_owner_invitations[0].expires_at,
            }],
            users: vec![owner.clone().into(), user.as_model().clone().into()],
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_does_not_include_expired_invites_v1() {
    let (app, _, owner, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    let user = app.db_new_user("invited_user").await;

    let krate1 = CrateBuilder::new("invited_crate_1", owner.id)
        .expect_build(&mut conn)
        .await;
    let krate2 = CrateBuilder::new("invited_crate_2", owner.id)
        .expect_build(&mut conn)
        .await;
    token
        .add_named_owner("invited_crate_1", "invited_user")
        .await
        .good();
    token
        .add_named_owner("invited_crate_2", "invited_user")
        .await
        .good();

    // Simulate one of the invitations expiring
    expire_invitation(&app, krate1.id).await;

    let invitations = user.list_invitations().await;
    assert_eq!(
        invitations,
        InvitationListResponse {
            crate_owner_invitations: vec![EncodableCrateOwnerInvitationV1 {
                crate_id: krate2.id,
                crate_name: krate2.name,
                invited_by_username: owner.gh_login.clone(),
                invitee_id: user.as_model().id,
                inviter_id: owner.id,
                // This value changes with each test run so we can't use a fixed value here
                created_at: invitations.crate_owner_invitations[0].created_at,
                // This value changes with each test run so we can't use a fixed value here
                expires_at: invitations.crate_owner_invitations[0].expires_at,
            }],
            users: vec![owner.clone().into(), user.as_model().clone().into()],
        }
    );
}

/// Given a user inviting a different user to be a crate
/// owner, check that the user invited can accept their
/// invitation, the invitation will be deleted from
/// the invitations table, and a new crate owner will be
/// inserted into the table for the given crate.
#[tokio::test(flavor = "multi_thread")]
async fn test_accept_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar").await;

    let krate = CrateBuilder::new("accept_invitation", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new owner
    owner_token
        .add_named_owner("accept_invitation", "user_bar")
        .await
        .good();

    // New owner accepts the invitation
    invited_user
        .accept_ownership_invitation(&krate.name, krate.id)
        .await;

    // New owner's invitation list should now be empty
    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // New owner is now listed as an owner, so the crate has two owners
    let json = anon.show_crate_owners("accept_invitation").await;
    assert_eq!(json.users.len(), 2);
}

/// Given a user inviting a different user to be a crate
/// owner, check that the user invited can decline their
/// invitation and the invitation will be deleted from
/// the invitations table.
#[tokio::test(flavor = "multi_thread")]
async fn test_decline_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar").await;

    let krate = CrateBuilder::new("decline_invitation", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new owner
    owner_token
        .add_named_owner("decline_invitation", "user_bar")
        .await
        .good();

    // Invited user declines the invitation
    invited_user
        .decline_ownership_invitation(&krate.name, krate.id)
        .await;

    // Invited user's invitation list should now be empty
    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("decline_invitation").await;
    assert_eq!(json.users.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_accept_invitation_by_mail() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar").await;

    CrateBuilder::new("accept_invitation", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new owner
    owner_token
        .add_named_owner("accept_invitation", "user_bar")
        .await
        .good();

    // Retrieve the ownership invitation
    let invite_token = extract_token_from_invite_email(&app.emails().await);

    // Accept the invitation anonymously with a token
    anon.accept_ownership_invitation_by_token(&invite_token)
        .await;

    // New owner's invitation list should now be empty
    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // New owner is now listed as an owner, so the crate has two owners
    let json = anon.show_crate_owners("accept_invitation").await;
    assert_eq!(json.users.len(), 2);
}

/// Hacky way to simulate the expiration of an ownership invitation. Instead of letting a month
/// pass, the creation date of the invite is moved back a month.
pub async fn expire_invitation(app: &TestApp, crate_id: i32) {
    use crates_io::schema::crate_owner_invitations;

    let mut conn = app.db_conn().await;

    let expiration = app.as_inner().config.ownership_invitations_expiration;

    let now = Utc::now();
    let created_at = (now - expiration).naive_utc();

    diesel::update(crate_owner_invitations::table)
        .set((
            crate_owner_invitations::created_at.eq(created_at),
            crate_owner_invitations::expires_at.eq(now),
        ))
        .filter(crate_owner_invitations::crate_id.eq(crate_id))
        .execute(&mut conn)
        .await
        .expect("failed to override the creation time");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_accept_expired_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();
    let invited_user = app.db_new_user("demo_user").await;

    let krate = CrateBuilder::new("demo_crate", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new user
    owner_token
        .add_named_owner("demo_crate", "demo_user")
        .await
        .good();

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id).await;

    // New owner tries to accept the invitation but it fails
    let resp = invited_user
        .try_accept_ownership_invitation::<()>(&krate.name, krate.id)
        .await;
    assert_eq!(resp.status(), StatusCode::GONE);
    assert_eq!(
        resp.json(),
        json!({
            "errors": [
                {
                    "detail": "The invitation to become an owner of the demo_crate crate expired. \
                               Please reach out to an owner of the crate to request a new invitation.",
                }
            ]
        })
    );

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate").await;
    assert_eq!(json.users.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_decline_expired_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();
    let invited_user = app.db_new_user("demo_user").await;

    let krate = CrateBuilder::new("demo_crate", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new user
    owner_token
        .add_named_owner("demo_crate", "demo_user")
        .await
        .good();

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id).await;

    // New owner declines the invitation and it succeeds, even though the invitation expired.
    invited_user
        .decline_ownership_invitation(&krate.name, krate.id)
        .await;

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate").await;
    assert_eq!(json.users.len(), 1);
}

/// Given a user inviting a different user to be a crate
/// owner, check that the user invited cannot accept their
/// invitation if they don't have a verified email address.
#[tokio::test(flavor = "multi_thread")]
async fn test_accept_invitation_without_verified_email() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    // Create a user with a verified email (default behavior of db_new_user)
    let invited_user = app.db_new_user("user_unverified").await;

    // Update the email to be unverified
    diesel::update(emails::table)
        .filter(emails::user_id.eq(invited_user.as_model().id))
        .set(emails::verified.eq(false))
        .execute(&mut conn)
        .await
        .unwrap();

    let krate = CrateBuilder::new("foo", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite the unverified user
    owner_token
        .add_named_owner("foo", "user_unverified")
        .await
        .good();

    // Attempt to accept the invitation - this should fail
    let response = invited_user
        .try_accept_ownership_invitation::<()>(&krate.name, krate.id)
        .await;

    // Verify that the response is a 403 Forbidden with the expected error message
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You need to verify your email address before you can accept the invitation to become an owner of the foo crate."}]}"#);

    // Verify that the invitation still exists
    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 1);

    // Verify that the user is not listed as an owner
    let json = anon.show_crate_owners("foo").await;
    assert_eq!(json.users.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_accept_expired_invitation_by_mail() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    let owner = owner.as_model();
    let _invited_user = app.db_new_user("demo_user").await;
    let krate = CrateBuilder::new("demo_crate", owner.id)
        .expect_build(&mut conn)
        .await;

    // Invite a new owner
    owner_token
        .add_named_owner("demo_crate", "demo_user")
        .await
        .good();

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id).await;

    // Retrieve the ownership invitation
    let invite_token = extract_token_from_invite_email(&app.emails().await);

    // Try to accept the invitation, and ensure it fails.
    let resp = anon
        .try_accept_ownership_invitation_by_token::<()>(&invite_token)
        .await;
    assert_eq!(resp.status(), StatusCode::GONE);
    assert_eq!(
        resp.json(),
        json!({
            "errors": [
                {
                    "detail": "The invitation to become an owner of the demo_crate crate expired. \
                               Please reach out to an owner of the crate to request a new invitation.",
                }
            ]
        })
    );

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate").await;
    assert_eq!(json.users.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn inactive_users_dont_get_invitations() {
    use crates_io::models::NewUser;

    let (app, _, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    // An inactive user with gh_id -1 and an active user with a non-negative gh_id both exist
    let invited_gh_login = "user_bar";
    let krate_name = "inactive_test";

    NewUser::builder()
        .gh_id(-1)
        .gh_login(invited_gh_login)
        .gh_encrypted_token(&[])
        .build()
        .insert(&mut conn)
        .await
        .unwrap();

    CrateBuilder::new(krate_name, owner.id)
        .expect_build(&mut conn)
        .await;

    let invited_user = app.db_new_user(invited_gh_login).await;

    owner_token
        .add_named_owner(krate_name, "user_bar")
        .await
        .good();

    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn highest_gh_id_is_most_recent_account_we_know_of() {
    let (app, _, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    // An inactive user with a lower gh_id and an active user with a higher gh_id both exist
    let invited_gh_login = "user_bar";
    let krate_name = "newer_user_test";

    // This user will get a lower gh_id, given how crate::tests::new_user works
    app.db_new_user(invited_gh_login).await;

    let invited_user = app.db_new_user(invited_gh_login).await;

    CrateBuilder::new(krate_name, owner.id)
        .expect_build(&mut conn)
        .await;

    owner_token
        .add_named_owner(krate_name, "user_bar")
        .await
        .good();

    let json = invited_user.list_invitations().await;
    assert_eq!(json.crate_owner_invitations.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn inviting_nonexistent_user_fails() {
    use crates_io::models::NewUser;

    let (app, _, owner, owner_token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let owner = owner.as_model();

    // An inactive user with gh_id -1 exists
    let invited_gh_login = "user_bar";
    NewUser::builder()
        .gh_id(-1)
        .gh_login(invited_gh_login)
        .gh_encrypted_token(&[])
        .build()
        .insert(&mut conn)
        .await
        .unwrap();

    let krate_name = "inactive_test";
    CrateBuilder::new(krate_name, owner.id)
        .expect_build(&mut conn)
        .await;

    // But trying to add the inactive user fails
    let response = owner_token
        .add_named_owner(krate_name, invited_gh_login)
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"could not find user with login `user_bar`"}]}"#
    );

    // Adding a user that doesn't exist at all also fails
    let response = owner_token
        .add_named_owner(krate_name, "nonexistent_username")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"could not find user with login `nonexistent_username`"}]}"#
    );
}

fn extract_token_from_invite_email(emails: &[String]) -> String {
    let body = emails
        .iter()
        .find(|m| m.contains("Subject: crates.io: Ownership invitation"))
        .expect("missing email");

    // Simple (but kinda fragile) parser to extract the token.
    let before_token = "/accept-invite/";
    let after_token = " ";
    let before_pos = body.find(before_token).unwrap() + before_token.len();
    let after_pos = before_pos + body[before_pos..].find(after_token).unwrap();
    body[before_pos..after_pos].to_string()
}
