use crate::{
    add_team_to_crate,
    builders::{CrateBuilder, PublishBuilder},
    new_team,
    util::{MockAnonymousUser, MockCookieUser, MockTokenUser, RequestHelper, Response},
    TestApp,
};
use cargo_registry::{
    models::Crate,
    views::{
        EncodableCrateOwnerInvitation, EncodableCrateOwnerInvitationV1, EncodableOwner,
        EncodablePublicUser, InvitationResponse,
    },
    Emails,
};

use chrono::{Duration, Utc};
use conduit::StatusCode;
use diesel::prelude::*;

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
#[derive(Deserialize, Debug, PartialEq, Eq)]
struct CrateOwnerInvitationsResponse {
    invitations: Vec<EncodableCrateOwnerInvitation>,
    users: Vec<EncodablePublicUser>,
    meta: CrateOwnerInvitationsMeta,
}
#[derive(Deserialize, Debug, PartialEq, Eq)]
struct CrateOwnerInvitationsMeta {
    next_page: Option<String>,
}

// Implementing locally for now, unless these are needed elsewhere
impl MockCookieUser {
    fn try_accept_ownership_invitation<T: serde::de::DeserializeOwned>(
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
        self.put(&url, body.to_string().as_bytes())
    }

    /// As the currently logged in user, accept an invitation to become an owner of the named
    /// crate.
    fn accept_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
        #[derive(Deserialize)]
        struct CrateOwnerInvitation {
            crate_owner_invitation: InvitationResponse,
        }

        let crate_owner_invite: CrateOwnerInvitation = self
            .try_accept_ownership_invitation(krate_name, krate_id)
            .good();

        assert!(crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// As the currently logged in user, decline an invitation to become an owner of the named
    /// crate.
    fn decline_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
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
            self.put(&url, body.to_string().as_bytes()).good();
        assert!(!crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// As the currently logged in user, list my pending invitations.
    fn list_invitations(&self) -> InvitationListResponse {
        self.get("/api/v1/me/crate_owner_invitations").good()
    }
}

impl MockAnonymousUser {
    fn accept_ownership_invitation_by_token(&self, token: &str) {
        #[derive(Deserialize)]
        struct Response {
            crate_owner_invitation: InvitationResponse,
        }

        let response: Response = self.try_accept_ownership_invitation_by_token(token).good();
        assert!(response.crate_owner_invitation.accepted);
    }

    fn try_accept_ownership_invitation_by_token<T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
    ) -> Response<T> {
        let url = format!("/api/v1/me/crate_owner_invitations/accept/{token}");
        self.put(&url, &[])
    }
}

#[test]
fn new_crate_owner() {
    let (app, _, _, token) = TestApp::full().with_token();

    // Create a crate under one user
    let crate_to_publish = PublishBuilder::new("foo_owner").version("1.0.0");
    token.enqueue_publish(crate_to_publish).good();

    // Add the second user as an owner (with a different case to make sure that works)
    let user2 = app.db_new_user("Bar");
    token.add_user_owner("foo_owner", "BAR");

    // accept invitation for user to be added as owner
    let krate: Crate = app.db(|conn| Crate::by_name("foo_owner").first(conn).unwrap());
    user2.accept_ownership_invitation("foo_owner", krate.id);

    // Make sure this shows up as one of their crates.
    let crates = user2.search_by_user_id(user2.as_model().id);
    assert_eq!(crates.crates.len(), 1);

    // And upload a new version as the second user
    let crate_to_publish = PublishBuilder::new("foo_owner").version("2.0.0");
    user2
        .db_new_token("bar_token")
        .enqueue_publish(crate_to_publish)
        .good();
}

fn create_and_add_owner(
    app: &TestApp,
    token: &MockTokenUser,
    username: &str,
    krate: &Crate,
) -> MockCookieUser {
    let user = app.db_new_user(username);
    token.add_user_owner(&krate.name, username);
    user.accept_ownership_invitation(&krate.name, krate.id);
    user
}

// Ensures that so long as at least one owner remains associated with the crate,
// a user can still remove their own login as an owner
#[test]
fn owners_can_remove_self() {
    let (app, _, user, token) = TestApp::init().with_token();
    let username = &user.as_model().gh_login;

    let krate = app
        .db(|conn| CrateBuilder::new("owners_selfremove", user.as_model().id).expect_build(conn));

    // Deleting yourself when you're the only owner isn't allowed.
    let response = token.remove_named_owner("owners_selfremove", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required." }] })
    );

    create_and_add_owner(&app, &token, "secondowner", &krate);

    // Deleting yourself when there are other owners is allowed.
    let response = token.remove_named_owner("owners_selfremove", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "msg": "owners successfully removed", "ok": true })
    );

    // After you delete yourself, you no longer have permisions to manage the crate.
    let response = token.remove_named_owner("owners_selfremove", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "only owners have permission to modify owners" }] })
    );
}

// Verify consistency when adidng or removing multiple owners in a single request.
#[test]
fn modify_multiple_owners() {
    let (app, _, user, token) = TestApp::init().with_token();
    let username = &user.as_model().gh_login;

    let krate =
        app.db(|conn| CrateBuilder::new("owners_multiple", user.as_model().id).expect_build(conn));

    let user2 = create_and_add_owner(&app, &token, "user2", &krate);
    let user3 = create_and_add_owner(&app, &token, "user3", &krate);

    // Deleting all owners is not allowed.
    let response = token.remove_named_owners("owners_multiple", &[username, "user2", "user3"]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required." }] })
    );
    assert_eq!(app.db(|conn| krate.owners(conn).unwrap()).len(), 3);

    // Deleting two owners at once is allowed.
    let response = token.remove_named_owners("owners_multiple", &["user2", "user3"]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "msg": "owners successfully removed", "ok": true })
    );
    assert_eq!(app.db(|conn| krate.owners(conn).unwrap()).len(), 1);

    // Adding multiple users fails if one of them already is an owner.
    let response = token.add_named_owners("owners_multiple", &["user2", username]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "`foo` is already an owner" }] })
    );
    assert_eq!(app.db(|conn| krate.owners(conn).unwrap()).len(), 1);

    // Adding multiple users at once succeeds.
    let response = token.add_named_owners("owners_multiple", &["user2", "user3"]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({
            "msg": "user user2 has been invited to be an owner of crate owners_multiple,user user3 has been invited to be an owner of crate owners_multiple",
            "ok": true,
        })
    );

    user2.accept_ownership_invitation(&krate.name, krate.id);
    user3.accept_ownership_invitation(&krate.name, krate.id);

    assert_eq!(app.db(|conn| krate.owners(conn).unwrap()).len(), 3);
}

#[test]
fn invite_already_invited_user() {
    let (app, _, _, owner) = TestApp::init().with_token();
    app.db_new_user("invited_user");
    app.db(|conn| CrateBuilder::new("crate_name", owner.as_model().user_id).expect_build(conn));

    // Ensure no emails were sent up to this point
    assert_eq!(0, app.as_inner().emails.mails_in_memory().unwrap().len());

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({
            "msg": "user invited_user has been invited to be an owner of crate crate_name",
            "ok": true,
        })
    );

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(1, app.as_inner().emails.mails_in_memory().unwrap().len());

    // Then invite the user a second time, the message should point out the user is already invited
    let response = owner.add_named_owner("crate_name", "invited_user");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({
            "msg": "user invited_user already has a pending invitation to be an owner of crate crate_name",
            "ok": true,
        })
    );

    // Check that no new email is sent after the second invitation
    assert_eq!(1, app.as_inner().emails.mails_in_memory().unwrap().len());
}

#[test]
fn invite_with_existing_expired_invite() {
    let (app, _, _, owner) = TestApp::init().with_token();
    app.db_new_user("invited_user");
    let krate =
        app.db(|conn| CrateBuilder::new("crate_name", owner.as_model().user_id).expect_build(conn));

    // Ensure no emails were sent up to this point
    assert_eq!(0, app.as_inner().emails.mails_in_memory().unwrap().len());

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({
            "msg": "user invited_user has been invited to be an owner of crate crate_name",
            "ok": true,
        })
    );

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(1, app.as_inner().emails.mails_in_memory().unwrap().len());

    // Simulate the previous invite expiring
    expire_invitation(&app, krate.id);

    // Then invite the user a second time, a new invite is created as the old one expired
    let response = owner.add_named_owner("crate_name", "invited_user");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({
            "msg": "user invited_user has been invited to be an owner of crate crate_name",
            "ok": true,
        })
    );

    // Check that the email for the second invite was sent
    assert_eq!(2, app.as_inner().emails.mails_in_memory().unwrap().len());
}

/*  Testing the crate ownership between two crates and one team.
    Given two crates, one crate owned by both a team and a user,
    one only owned by a user, check that the CrateList returned
    for the user_id contains only the crates owned by that user,
    and that the CrateList returned for the team_id contains
    only crates owned by that team.
*/
#[test]
fn check_ownership_two_crates() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let (krate_owned_by_team, team) = app.db(|conn| {
        let t = new_team("team_foo").create_or_update(conn).unwrap();
        let krate = CrateBuilder::new("foo", user.id).expect_build(conn);
        add_team_to_crate(&t, &krate, user, conn).unwrap();
        (krate, t)
    });

    let user2 = app.db_new_user("user_bar");
    let user2 = user2.as_model();
    let krate_not_owned_by_team =
        app.db(|conn| CrateBuilder::new("bar", user2.id).expect_build(conn));

    let json = anon.search_by_user_id(user2.id);
    assert_eq!(json.crates[0].name, krate_not_owned_by_team.name);
    assert_eq!(json.crates.len(), 1);

    let query = format!("team_id={}", team.id);
    let json = anon.search(&query);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.crates[0].name, krate_owned_by_team.name);
}

/*  Given a crate owned by both a team and a user, check that the
    JSON returned by the /owner_team route and /owner_user route
    contains the correct kind of owner

    Note that in this case function new_team must take a team name
    of form github:org_name:team_name as that is the format
    EncodableOwner::encodable is expecting
*/
#[test]
fn check_ownership_one_crate() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let team = app.db(|conn| {
        let t = new_team("github:test_org:team_sloth")
            .create_or_update(conn)
            .unwrap();
        let krate = CrateBuilder::new("best_crate", user.id).expect_build(conn);
        add_team_to_crate(&t, &krate, user, conn).unwrap();
        t
    });

    let json: TeamResponse = anon.get("/api/v1/crates/best_crate/owner_team").good();
    assert_eq!(json.teams[0].kind, "team");
    assert_eq!(json.teams[0].name, team.name);

    let json: UserResponse = anon.get("/api/v1/crates/best_crate/owner_user").good();
    assert_eq!(json.users[0].kind, "user");
    assert_eq!(json.users[0].name, user.name);
}

#[test]
fn deleted_ownership_isnt_in_owner_user() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user.id).expect_build(conn);
        krate
            .owner_remove(app.as_inner(), conn, user, &user.gh_login)
            .unwrap();
    });

    let json: UserResponse = anon.get("/api/v1/crates/foo_my_packages/owner_user").good();
    assert_eq!(json.users.len(), 0);
}

#[test]
fn invitations_are_empty_by_default_v1() {
    let (_, _, user) = TestApp::init().with_user();

    let json = user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 0);
}

#[test]
fn api_token_cannot_list_invitations_v1() {
    let (_, _, _, token) = TestApp::init().with_token();

    token
        .get("/api/v1/me/crate_owner_invitations")
        .assert_forbidden();
}

#[test]
fn invitations_list_v1() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let owner = owner.as_model();

    let krate = app.db(|conn| CrateBuilder::new("invited_crate", owner.id).expect_build(conn));

    let user = app.db_new_user("invited_user");
    token.add_user_owner("invited_crate", "invited_user");

    let response = user.get::<()>("/api/v1/me/crate_owner_invitations");
    assert_eq!(response.status(), StatusCode::OK);

    let invitations = user.list_invitations();
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

#[test]
fn invitations_list_does_not_include_expired_invites_v1() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let owner = owner.as_model();

    let user = app.db_new_user("invited_user");

    let krate1 = app.db(|conn| CrateBuilder::new("invited_crate_1", owner.id).expect_build(conn));
    let krate2 = app.db(|conn| CrateBuilder::new("invited_crate_2", owner.id).expect_build(conn));
    token.add_user_owner("invited_crate_1", "invited_user");
    token.add_user_owner("invited_crate_2", "invited_user");

    // Simulate one of the invitations expiring
    expire_invitation(&app, krate1.id);

    let invitations = user.list_invitations();
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

/*  Given a user inviting a different user to be a crate
    owner, check that the user invited can accept their
    invitation, the invitation will be deleted from
    the invitations table, and a new crate owner will be
    inserted into the table for the given crate.
*/
#[test]
fn test_accept_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar");
    let krate = app.db(|conn| CrateBuilder::new("accept_invitation", owner.id).expect_build(conn));

    // Invite a new owner
    owner_token.add_user_owner("accept_invitation", "user_bar");

    // New owner accepts the invitation
    invited_user.accept_ownership_invitation(&krate.name, krate.id);

    // New owner's invitation list should now be empty
    let json = invited_user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // New owner is now listed as an owner, so the crate has two owners
    let json = anon.show_crate_owners("accept_invitation");
    assert_eq!(json.users.len(), 2);
}

/*  Given a user inviting a different user to be a crate
    owner, check that the user invited can decline their
    invitation and the invitation will be deleted from
    the invitations table.
*/
#[test]
fn test_decline_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar");
    let krate = app.db(|conn| CrateBuilder::new("decline_invitation", owner.id).expect_build(conn));

    // Invite a new owner
    owner_token.add_user_owner("decline_invitation", "user_bar");

    // Invited user declines the invitation
    invited_user.decline_ownership_invitation(&krate.name, krate.id);

    // Invited user's invitation list should now be empty
    let json = invited_user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("decline_invitation");
    assert_eq!(json.users.len(), 1);
}

#[test]
fn test_accept_invitation_by_mail() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let invited_user = app.db_new_user("user_bar");
    let _krate = app.db(|conn| CrateBuilder::new("accept_invitation", owner.id).expect_build(conn));

    // Invite a new owner
    owner_token.add_user_owner("accept_invitation", "user_bar");

    // Retrieve the ownership invitation
    let invite_token = extract_token_from_invite_email(&app.as_inner().emails);

    // Accept the invitation anonymously with a token
    anon.accept_ownership_invitation_by_token(&invite_token);

    // New owner's invitation list should now be empty
    let json = invited_user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // New owner is now listed as an owner, so the crate has two owners
    let json = anon.show_crate_owners("accept_invitation");
    assert_eq!(json.users.len(), 2);
}

/// Hacky way to simulate the expiration of an ownership invitation. Instead of letting a month
/// pass, the creation date of the invite is moved back a month.
fn expire_invitation(app: &TestApp, crate_id: i32) {
    use cargo_registry::schema::crate_owner_invitations;

    app.db(|conn| {
        let expiration = app.as_inner().config.ownership_invitations_expiration_days as i64;
        let created_at = (Utc::now() - Duration::days(expiration)).naive_utc();

        diesel::update(crate_owner_invitations::table)
            .set(crate_owner_invitations::created_at.eq(created_at))
            .filter(crate_owner_invitations::crate_id.eq(crate_id))
            .execute(conn)
            .expect("failed to override the creation time");
    });
}

#[test]
fn test_accept_expired_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let invited_user = app.db_new_user("demo_user");
    let krate = app.db(|conn| CrateBuilder::new("demo_crate", owner.id).expect_build(conn));

    // Invite a new user
    owner_token.add_user_owner("demo_crate", "demo_user");

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id);

    // New owner tries to accept the invitation but it fails
    let resp = invited_user.try_accept_ownership_invitation::<()>(&krate.name, krate.id);
    assert_eq!(StatusCode::GONE, resp.status());
    assert_eq!(
        json!({
            "errors": [
                {
                    "detail": "The invitation to become an owner of the demo_crate crate expired. \
                               Please reach out to an owner of the crate to request a new invitation.",
                }
            ]
        }),
        resp.into_json()
    );

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate");
    assert_eq!(json.users.len(), 1);
}

#[test]
fn test_decline_expired_invitation() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let invited_user = app.db_new_user("demo_user");
    let krate = app.db(|conn| CrateBuilder::new("demo_crate", owner.id).expect_build(conn));

    // Invite a new user
    owner_token.add_user_owner("demo_crate", "demo_user");

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id);

    // New owner declines the invitation and it succeeds, even though the invitation expired.
    invited_user.decline_ownership_invitation(&krate.name, krate.id);

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate");
    assert_eq!(json.users.len(), 1);
}

#[test]
fn test_accept_expired_invitation_by_mail() {
    let (app, anon, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();
    let _invited_user = app.db_new_user("demo_user");
    let krate = app.db(|conn| CrateBuilder::new("demo_crate", owner.id).expect_build(conn));

    // Invite a new owner
    owner_token.add_user_owner("demo_crate", "demo_user");

    // Manually update the creation time to simulate the invite expiring
    expire_invitation(&app, krate.id);

    // Retrieve the ownership invitation
    let invite_token = extract_token_from_invite_email(&app.as_inner().emails);

    // Try to accept the invitation, and ensure it fails.
    let resp = anon.try_accept_ownership_invitation_by_token::<()>(&invite_token);
    assert_eq!(StatusCode::GONE, resp.status());
    assert_eq!(
        json!({
            "errors": [
                {
                    "detail": "The invitation to become an owner of the demo_crate crate expired. \
                               Please reach out to an owner of the crate to request a new invitation.",
                }
            ]
        }),
        resp.into_json()
    );

    // Invited user is NOT listed as an owner, so the crate still only has one owner
    let json = anon.show_crate_owners("demo_crate");
    assert_eq!(json.users.len(), 1);
}

#[test]
fn inactive_users_dont_get_invitations() {
    use cargo_registry::models::NewUser;
    use std::borrow::Cow;

    let (app, _, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();

    // An inactive user with gh_id -1 and an active user with a non-negative gh_id both exist
    let invited_gh_login = "user_bar";
    let krate_name = "inactive_test";

    app.db(|conn| {
        NewUser {
            gh_id: -1,
            gh_login: invited_gh_login,
            name: None,
            gh_avatar: None,
            gh_access_token: Cow::Borrowed("some random token"),
        }
        .create_or_update(None, &app.as_inner().emails, conn)
        .unwrap();
        CrateBuilder::new(krate_name, owner.id).expect_build(conn);
    });

    let invited_user = app.db_new_user(invited_gh_login);

    owner_token.add_user_owner(krate_name, "user_bar");

    let json = invited_user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 1);
}

#[test]
fn highest_gh_id_is_most_recent_account_we_know_of() {
    let (app, _, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();

    // An inactive user with a lower gh_id and an active user with a higher gh_id both exist
    let invited_gh_login = "user_bar";
    let krate_name = "newer_user_test";

    // This user will get a lower gh_id, given how crate::new_user works
    app.db_new_user(invited_gh_login);

    let invited_user = app.db_new_user(invited_gh_login);

    app.db(|conn| {
        CrateBuilder::new(krate_name, owner.id).expect_build(conn);
    });

    owner_token.add_user_owner(krate_name, "user_bar");

    let json = invited_user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 1);
}

fn extract_token_from_invite_email(emails: &Emails) -> String {
    let message = emails
        .mails_in_memory()
        .unwrap()
        .into_iter()
        .find(|m| m.subject.contains("invitation"))
        .expect("missing email");

    // Simple (but kinda fragile) parser to extract the token.
    let before_token = "/accept-invite/";
    let after_token = " ";
    let body = message.body.as_str();
    let before_pos = body.find(before_token).unwrap() + before_token.len();
    let after_pos = before_pos + (&body[before_pos..]).find(after_token).unwrap();
    body[before_pos..after_pos].to_string()
}

//
// Tests for the `GET /api/private/crate-owners-invitations` endpoint
//

#[track_caller]
fn get_invitations(user: &MockCookieUser, query: &str) -> CrateOwnerInvitationsResponse {
    user.get_with_query::<CrateOwnerInvitationsResponse>(
        "/api/private/crate_owner_invitations",
        query,
    )
    .good()
}

#[test]
fn invitation_list() {
    let (app, _, owner, token) = TestApp::init().with_token();

    let (crate1, crate2) = app.db(|conn| {
        (
            CrateBuilder::new("crate_1", owner.as_model().id).expect_build(conn),
            CrateBuilder::new("crate_2", owner.as_model().id).expect_build(conn),
        )
    });
    let user1 = app.db_new_user("user_1");
    let user2 = app.db_new_user("user_2");
    token.add_user_owner("crate_1", "user_1");
    token.add_user_owner("crate_1", "user_2");
    token.add_user_owner("crate_2", "user_1");

    // user1 has invites for both crates
    let invitations = get_invitations(&user1, &format!("invitee_id={}", user1.as_model().id));
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[0].created_at,
                    expires_at: invitations.invitations[0].expires_at,
                },
                EncodableCrateOwnerInvitation {
                    crate_id: crate2.id,
                    crate_name: crate2.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[1].created_at,
                    expires_at: invitations.invitations[1].expires_at,
                },
            ],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into()
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // user2 is only invited to a single crate
    let invitations = get_invitations(&user2, &format!("invitee_id={}", user2.as_model().id));
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate1.id,
                crate_name: crate1.name.clone(),
                invitee_id: user2.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user2.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // owner has no invites
    let invitations = get_invitations(&owner, &format!("invitee_id={}", owner.as_model().id));
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![],
            users: vec![],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // crate1 has two available invitations
    let invitations = get_invitations(&owner, "crate_name=crate_1");
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[0].created_at,
                    expires_at: invitations.invitations[0].expires_at,
                },
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name,
                    invitee_id: user2.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[1].created_at,
                    expires_at: invitations.invitations[1].expires_at,
                },
            ],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into(),
                user2.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // crate2 has one available invitation
    let invitations = get_invitations(&owner, "crate_name=crate_2");
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user1.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[test]
fn invitations_list_does_not_include_expired_invites() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let user = app.db_new_user("invited_user");

    let (crate1, crate2) = app.db(|conn| {
        (
            CrateBuilder::new("crate_1", owner.as_model().id).expect_build(conn),
            CrateBuilder::new("crate_2", owner.as_model().id).expect_build(conn),
        )
    });
    token.add_user_owner("crate_1", "invited_user");
    token.add_user_owner("crate_2", "invited_user");

    // Simulate one of the invitations expiring
    expire_invitation(&app, crate1.id);

    // user1 has an invite just for crate 2
    let invitations = get_invitations(&user, &format!("invitee_id={}", user.as_model().id));
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[test]
fn invitations_list_paginated() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let user = app.db_new_user("invited_user");

    let (crate1, crate2) = app.db(|conn| {
        (
            CrateBuilder::new("crate_1", owner.as_model().id).expect_build(conn),
            CrateBuilder::new("crate_2", owner.as_model().id).expect_build(conn),
        )
    });
    token.add_user_owner("crate_1", "invited_user");
    token.add_user_owner("crate_2", "invited_user");

    // Fetch the first page of results
    let invitations = get_invitations(
        &user,
        &format!("per_page=1&invitee_id={}", user.as_model().id),
    );
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate1.id,
                crate_name: crate1.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta {
                // This unwraps and then wraps again in Some() to ensure it's not None
                next_page: Some(invitations.meta.next_page.clone().unwrap()),
            },
        }
    );

    // Fetch the second page of results
    let invitations = get_invitations(
        &user,
        invitations.meta.next_page.unwrap().trim_start_matches('?'),
    );
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[test]
fn invitation_list_with_no_filter() {
    let (_, _, owner, _) = TestApp::init().with_token();

    let resp = owner.get::<()>("/api/private/crate_owner_invitations");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.into_json(),
        json!({
            "errors": [{
                "detail": "missing or invalid filter",
            }],
        })
    );
}

#[test]
fn invitation_list_other_users() {
    let (app, _, owner, _) = TestApp::init().with_token();
    let other_user = app.db_new_user("other");

    // Retrieving our own invitations work.
    let resp = owner.get_with_query::<()>(
        "/api/private/crate_owner_invitations",
        &format!("invitee_id={}", owner.as_model().id),
    );
    assert_eq!(resp.status(), StatusCode::OK);

    // Retrieving other users' invitations doesn't work.
    let resp = owner.get_with_query::<()>(
        "/api/private/crate_owner_invitations",
        &format!("invitee_id={}", other_user.as_model().id),
    );
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[test]
fn invitation_list_other_crates() {
    let (app, _, owner, _) = TestApp::init().with_token();
    let other_user = app.db_new_user("other");
    app.db(|conn| {
        CrateBuilder::new("crate_1", owner.as_model().id).expect_build(conn);
        CrateBuilder::new("crate_2", other_user.as_model().id).expect_build(conn);
    });

    // Retrieving our own invitations work.
    let resp =
        owner.get_with_query::<()>("/api/private/crate_owner_invitations", "crate_name=crate_1");
    assert_eq!(resp.status(), StatusCode::OK);

    // Retrieving other users' invitations doesn't work.
    let resp =
        owner.get_with_query::<()>("/api/private/crate_owner_invitations", "crate_name=crate_2");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
