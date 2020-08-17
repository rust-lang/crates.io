use crate::{
    add_team_to_crate,
    builders::{CrateBuilder, PublishBuilder},
    new_team,
    util::{MockAnonymousUser, MockCookieUser, MockTokenUser, RequestHelper},
    TestApp,
};
use cargo_registry::{
    models::Crate,
    views::{EncodableCrateOwnerInvitation, EncodableOwner, InvitationResponse},
    Emails,
};

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
#[derive(Deserialize)]
struct InvitationListResponse {
    crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
}

// Implementing locally for now, unless these are needed elsewhere
impl MockCookieUser {
    /// As the currently logged in user, accept an invitation to become an owner of the named
    /// crate.
    fn accept_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
        let body = json!({
            "crate_owner_invite": {
                "invited_by_username": "",
                "crate_name": krate_name,
                "crate_id": krate_id,
                "created_at": "",
                "accepted": true
            }
        });

        #[derive(Deserialize)]
        struct CrateOwnerInvitation {
            crate_owner_invitation: InvitationResponse,
        }

        let url = format!("/api/v1/me/crate_owner_invitations/{}", krate_id);
        let crate_owner_invite: CrateOwnerInvitation =
            self.put(&url, body.to_string().as_bytes()).good();
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

        let url = format!("/api/v1/me/crate_owner_invitations/{}", krate_id);
        let crate_owner_invite: CrateOwnerInvitation =
            self.put(&url, body.to_string().as_bytes()).good();
        assert!(!crate_owner_invite.crate_owner_invitation.accepted);
        assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);
    }

    /// As the currently logged in user, list my pending invitations.
    fn list_invitations(&self) -> InvitationListResponse {
        self.get("/api/v1/me/crate_owner_invitations").good()
    }

    fn set_email_notifications(&self, krate_id: i32, email_notifications: bool) {
        let body = json!([
            {
                "id": krate_id,
                "email_notifications": email_notifications,
            }
        ]);

        #[derive(Deserialize)]
        struct Empty {}

        let _: Empty = self
            .put(
                "/api/v1/me/email_notifications",
                body.to_string().as_bytes(),
            )
            .good();
    }
}

impl MockAnonymousUser {
    fn accept_ownership_invitation_by_token(&self, token: &str) {
        #[derive(Deserialize)]
        struct Response {
            crate_owner_invitation: InvitationResponse,
        }

        let url = format!("/api/v1/me/crate_owner_invitations/accept/{}", token);
        let response: Response = self.put(&url, &[]).good();
        assert!(response.crate_owner_invitation.accepted);
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
        response.json(),
        json!({ "errors": [{ "detail": "cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required." }] })
    );

    create_and_add_owner(&app, &token, "secondowner", &krate);

    // Deleting yourself when there are other owners is allowed.
    let response = token.remove_named_owner("owners_selfremove", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "msg": "owners successfully removed", "ok": true })
    );

    // After you delete yourself, you no longer have permisions to manage the crate.
    let response = token.remove_named_owner("owners_selfremove", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
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
        response.json(),
        json!({ "errors": [{ "detail": "cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required." }] })
    );
    assert_eq!(app.db(|conn| krate.owners(&conn).unwrap()).len(), 3);

    // Deleting two owners at once is allowed.
    let response = token.remove_named_owners("owners_multiple", &["user2", "user3"]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "msg": "owners successfully removed", "ok": true })
    );
    assert_eq!(app.db(|conn| krate.owners(&conn).unwrap()).len(), 1);

    // Adding multiple users fails if one of them already is an owner.
    let response = token.add_named_owners("owners_multiple", &["user2", username]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "`foo` is already an owner" }] })
    );
    assert_eq!(app.db(|conn| krate.owners(&conn).unwrap()).len(), 1);

    // Adding multiple users at once succeeds.
    let response = token.add_named_owners("owners_multiple", &["user2", "user3"]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({
            "msg": "user user2 has been invited to be an owner of crate owners_multiple,user user3 has been invited to be an owner of crate owners_multiple",
            "ok": true,
        })
    );

    user2.accept_ownership_invitation(&krate.name, krate.id);
    user3.accept_ownership_invitation(&krate.name, krate.id);

    assert_eq!(app.db(|conn| krate.owners(&conn).unwrap()).len(), 3);
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
fn invitations_are_empty_by_default() {
    let (_, _, user) = TestApp::init().with_user();

    let json = user.list_invitations();
    assert_eq!(json.crate_owner_invitations.len(), 0);
}

#[test]
fn invitations_list() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let owner = owner.as_model();

    let krate = app.db(|conn| CrateBuilder::new("invited_crate", owner.id).expect_build(conn));

    let user = app.db_new_user("invited_user");
    token.add_user_owner("invited_crate", "invited_user");

    let response = user.get::<()>("/api/v1/me/crate_owner_invitations");
    assert_eq!(response.status(), StatusCode::OK);

    let json = response.json();
    assert_eq!(
        json,
        json!({
            "crate_owner_invitations": [{
                "crate_id": krate.id,
                "crate_name": "invited_crate",
                // this value changes with each test run so we can't use a fixed value here
                "created_at": &json["crate_owner_invitations"][0]["created_at"],
                "invited_by_username": owner.gh_login,
                "invitee_id": user.as_model().id,
                "inviter_id": owner.id,
            }],
            "users": [{
                "avatar": null,
                "id": owner.id,
                "login": "foo",
                "name": null,
                "url": "https://github.com/foo",
            }],
        })
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

#[test]
fn test_list_owners_with_notification_email() {
    let (app, _, owner, owner_token) = TestApp::init().with_token();
    let owner = owner.as_model();

    let krate_name = "notification_crate";
    let user_name = "notification_user";

    let new_user = app.db_new_user(user_name);
    let krate = app.db(|conn| CrateBuilder::new(krate_name, owner.id).expect_build(conn));

    // crate author gets notified
    let (owners_notification, email) = app.db(|conn| {
        let owners_notification = krate.owners_with_notification_email(conn).unwrap();
        let email = owner.verified_email(conn).unwrap().unwrap();
        (owners_notification, email)
    });
    assert_eq!(owners_notification, [email.clone()]);

    // crate author and the new crate owner get notified
    owner_token.add_named_owner(krate_name, user_name).good();
    new_user.accept_ownership_invitation(&krate.name, krate.id);

    let (owners_notification, new_user_email) = app.db(|conn| {
        let new_user_email = new_user.as_model().verified_email(conn).unwrap().unwrap();
        let owners_notification = krate.owners_with_notification_email(conn).unwrap();
        (owners_notification, new_user_email)
    });
    assert_eq!(owners_notification, [email.clone(), new_user_email]);

    // crate owners who disabled notifications don't get notified
    new_user.set_email_notifications(krate.id, false);

    let owners_notification = app.db(|conn| krate.owners_with_notification_email(conn).unwrap());
    assert_eq!(owners_notification, [email]);
}
