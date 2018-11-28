use conduit::{Handler, Method};
use diesel;
use diesel::prelude::*;

use builders::{CrateBuilder, PublishBuilder};
use models::{Crate, NewCrateOwnerInvitation, User};
use schema::crate_owner_invitations;
use util::RequestHelper;
use views::{
    EncodableCrateOwnerInvitation, EncodableOwner, EncodablePublicUser, InvitationResponse,
};
use {add_team_to_crate, app, logout, new_team, new_user, req, sign_in_as, Bad, OkBool, TestApp};

#[derive(Deserialize)]
struct TeamResponse {
    teams: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
struct UserResponse {
    users: Vec<EncodableOwner>,
}

// Implementing locally for now, unless these are needed elsewhere
impl ::util::MockCookieUser {
    /// As the currently logged in user, accept an invitation to become an owner of the named
    /// crate.
    fn accept_ownership_invitation(&self, krate_name: &str, krate_id: i32) {
        use views::InvitationResponse;

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
}

impl ::util::MockTokenUser {
    /// Add a user as an owner for a crate.
    pub fn add_user_owner(&self, krate_name: &str, user: &User) {
        let url = format!("/api/v1/crates/{}/owners", krate_name);
        let body = format!("{{\"users\":[\"{}\"]}}", user.gh_login);

        let response: OkBool = self.put(&url, body.as_bytes()).good();
        assert!(response.ok);
    }
}

#[test]
fn new_crate_owner() {
    let (app, _, _, token) = TestApp::with_proxy().with_token();

    // Create a crate under one user
    let crate_to_publish = PublishBuilder::new("foo_owner").version("1.0.0");
    token.publish(crate_to_publish).good();

    // Add the second user as an owner
    let user2 = app.db_new_user("bar");
    token.add_user_owner("foo_owner", user2.as_model());

    // accept invitation for user to be added as owner
    let crate_id = app.db(|conn| Crate::by_name("foo_owner").first::<Crate>(conn).unwrap().id);
    user2.accept_ownership_invitation("foo_owner", crate_id);

    // Make sure this shows up as one of their crates.
    let crates = user2.search_by_user_id(user2.as_model().id);
    assert_eq!(crates.crates.len(), 1);

    // And upload a new crate as the second user
    let crate_to_publish = PublishBuilder::new("foo_owner").version("2.0.0");
    user2
        .db_new_token("bar_token")
        .publish(crate_to_publish)
        .good();
}

// Ensures that so long as at least one owner remains associated with the crate,
// a user can still remove their own login as an owner
#[test]
fn owners_can_remove_self() {
    #[derive(Deserialize)]
    struct R {
        users: Vec<EncodablePublicUser>,
    }

    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/crates/owners_selfremove/owners");
    let (first_owner, second_owner) = {
        let conn = app.diesel_database.get().unwrap();
        let user = new_user("firstowner").create_or_update(&conn).unwrap();
        let user_two = new_user("secondowner").create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        CrateBuilder::new("owners_selfremove", user.id).expect_build(&conn);
        (user, user_two)
    };

    let mut response = ok_resp!(middle.call(&mut req));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    // Deleting yourself when you're the only owner isn't allowed.
    let body = r#"{"users":["firstowner"]}"#;
    let mut response =
        ok_resp!(middle.call(req.with_method(Method::Delete,).with_body(body.as_bytes(),),));
    let json = ::json::<Bad>(&mut response);
    assert!(
        json.errors[0]
            .detail
            .contains("cannot remove the sole owner of a crate",)
    );

    let body = r#"{"users":["secondowner"]}"#;
    let mut response =
        ok_resp!(middle.call(req.with_method(Method::Put,).with_body(body.as_bytes(),),));
    assert!(::json::<OkBool>(&mut response).ok);

    // Need to accept owner invitation to add secondowner as owner
    let krate_id = {
        let conn = app.diesel_database.get().unwrap();
        Crate::by_name("owners_selfremove")
            .first::<Crate>(&*conn)
            .unwrap()
            .id
    };

    let body = json!({
        "crate_owner_invite": {
            "invited_by_username": "foo",
            "crate_name": "foo_owner",
            "crate_id": krate_id,
            "created_at": "",
            "accepted": true
        }
    });

    logout(&mut req);
    sign_in_as(&mut req, &second_owner);

    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/me/crate_owner_invitations/{}", krate_id))
                .with_method(Method::Put)
                .with_body(body.to_string().as_bytes()),
        )
    );

    #[derive(Deserialize)]
    struct CrateOwnerInvitation {
        crate_owner_invitation: InvitationResponse,
    }

    let crate_owner_invite = ::json::<CrateOwnerInvitation>(&mut response);
    assert!(crate_owner_invite.crate_owner_invitation.accepted);
    assert_eq!(crate_owner_invite.crate_owner_invitation.crate_id, krate_id);

    logout(&mut req);
    sign_in_as(&mut req, &first_owner);

    // Deleting yourself when there are other owners is allowed.
    let body = r#"{"users":["firstowner"]}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/owners_selfremove/owners")
                .with_method(Method::Delete)
                .with_body(body.as_bytes())
        )
    );
    assert!(::json::<OkBool>(&mut response).ok);

    // After you delete yourself, you no longer have permisions to manage the crate.
    let body = r#"{"users":["secondowner"]}"#;
    let mut response =
        ok_resp!(middle.call(req.with_method(Method::Delete,).with_body(body.as_bytes(),),));
    let json = ::json::<Bad>(&mut response);
    assert!(
        json.errors[0]
            .detail
            .contains("only owners have permission to modify owners",)
    );
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
        app.db(|conn| CrateBuilder::new("bar", user2.id).expect_build(&conn));

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
    let (_b, app, middle) = app();

    let (team, user) = {
        let conn = app.diesel_database.get().unwrap();
        let u = new_user("user_cat").create_or_update(&conn).unwrap();
        let t = new_team("github:test_org:team_sloth")
            .create_or_update(&conn)
            .unwrap();
        let krate = CrateBuilder::new("best_crate", u.id).expect_build(&conn);
        add_team_to_crate(&t, &krate, &u, &conn).unwrap();
        (t, u)
    };

    let mut req = req(Method::Get, "/api/v1/crates/best_crate/owner_team");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: TeamResponse = ::json(&mut response);

    assert_eq!(json.teams[0].kind, "team");
    assert_eq!(json.teams[0].name, team.name);

    let mut req = ::req(Method::Get, "/api/v1/crates/best_crate/owner_user");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserResponse = ::json(&mut response);

    assert_eq!(json.users[0].kind, "user");
    assert_eq!(json.users[0].name, user.name);
}

#[test]
fn invitations_are_empty_by_default() {
    #[derive(Deserialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me/crate_owner_invitations");

    let user = {
        let conn = app.diesel_database.get().unwrap();
        new_user("user_no_invites").create_or_update(&conn).unwrap()
    };
    sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: R = ::json(&mut response);

    assert_eq!(json.crate_owner_invitations.len(), 0);
}

#[test]
fn invitations_list() {
    #[derive(Deserialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me/crate_owner_invitations");
    let (krate, user) = {
        let conn = app.diesel_database.get().unwrap();
        let owner = new_user("inviting_user").create_or_update(&conn).unwrap();
        let user = new_user("invited_user").create_or_update(&conn).unwrap();
        let krate = CrateBuilder::new("invited_crate", owner.id).expect_build(&conn);

        // This should be replaced by an actual call to the route that `owner --add` hits once
        // that route creates an invitation.
        diesel::insert_into(crate_owner_invitations::table)
            .values(&NewCrateOwnerInvitation {
                invited_by_user_id: owner.id,
                invited_user_id: user.id,
                crate_id: krate.id,
            }).execute(&*conn)
            .unwrap();
        (krate, user)
    };
    sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: R = ::json(&mut response);

    assert_eq!(json.crate_owner_invitations.len(), 1);
    assert_eq!(
        json.crate_owner_invitations[0].invited_by_username,
        "inviting_user"
    );
    assert_eq!(json.crate_owner_invitations[0].crate_name, "invited_crate");
    assert_eq!(json.crate_owner_invitations[0].crate_id, krate.id);
}

/*  Given a user inviting a different user to be a crate
    owner, check that the user invited can accept their
    invitation, the invitation will be deleted from
    the invitations table, and a new crate owner will be
    inserted into the table for the given crate.
*/
#[test]
fn test_accept_invitation() {
    #[derive(Deserialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    #[derive(Deserialize)]
    struct Q {
        users: Vec<EncodablePublicUser>,
    }

    #[derive(Deserialize)]
    struct T {
        crate_owner_invitation: InvitationResponse,
    }

    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me/crate_owner_invitations");
    let (krate, user) = {
        let conn = app.diesel_database.get().unwrap();
        let owner = new_user("inviting_user").create_or_update(&conn).unwrap();
        let user = new_user("invited_user").create_or_update(&conn).unwrap();
        let krate = CrateBuilder::new("invited_crate", owner.id).expect_build(&conn);

        // This should be replaced by an actual call to the route that `owner --add` hits once
        // that route creates an invitation.
        diesel::insert_into(crate_owner_invitations::table)
            .values(&NewCrateOwnerInvitation {
                invited_by_user_id: owner.id,
                invited_user_id: user.id,
                crate_id: krate.id,
            }).execute(&*conn)
            .unwrap();
        (krate, user)
    };
    sign_in_as(&mut req, &user);

    let body = json!({
        "crate_owner_invite": {
            "invited_by_username": "inviting_user",
            "crate_name": "invited_crate",
            "crate_id": krate.id,
            "created_at": "",
            "accepted": true
        }
    });

    // first check that response from inserting new crate owner
    // and deleting crate_owner_invitation is okay
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("api/v1/me/crate_owner_invitations/{}", krate.id))
                .with_method(Method::Put)
                .with_body(body.to_string().as_bytes()),
        )
    );

    let json: T = ::json(&mut response);
    assert_eq!(json.crate_owner_invitation.accepted, true);
    assert_eq!(json.crate_owner_invitation.crate_id, krate.id);

    // then check to make sure that accept_invite did what it
    // was supposed to
    // crate_owner_invitation was deleted
    let mut response = ok_resp!(
        middle.call(
            req.with_path("api/v1/me/crate_owner_invitations")
                .with_method(Method::Get)
        )
    );
    let json: R = ::json(&mut response);
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // new crate owner was inserted
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/invited_crate/owners")
                .with_method(Method::Get)
        )
    );
    let json: Q = ::json(&mut response);
    assert_eq!(json.users.len(), 2);
}

/*  Given a user inviting a different user to be a crate
    owner, check that the user invited can decline their
    invitation and the invitation will be deleted from
    the invitations table.
*/
#[test]
fn test_decline_invitation() {
    #[derive(Deserialize)]
    struct R {
        crate_owner_invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    #[derive(Deserialize)]
    struct Q {
        users: Vec<EncodablePublicUser>,
    }

    #[derive(Deserialize)]
    struct T {
        crate_owner_invitation: InvitationResponse,
    }

    let (_b, app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me/crate_owner_invitations");
    let (krate, user) = {
        let conn = app.diesel_database.get().unwrap();
        let owner = new_user("inviting_user").create_or_update(&conn).unwrap();
        let user = new_user("invited_user").create_or_update(&conn).unwrap();
        let krate = CrateBuilder::new("invited_crate", owner.id).expect_build(&conn);

        // This should be replaced by an actual call to the route that `owner --add` hits once
        // that route creates an invitation.
        diesel::insert_into(crate_owner_invitations::table)
            .values(&NewCrateOwnerInvitation {
                invited_by_user_id: owner.id,
                invited_user_id: user.id,
                crate_id: krate.id,
            }).execute(&*conn)
            .unwrap();
        (krate, user)
    };
    sign_in_as(&mut req, &user);

    let body = json!({
        "crate_owner_invite": {
            "invited_by_username": "inviting_user",
            "crate_name": "invited_crate",
            "crate_id": krate.id,
            "created_at": "",
            "accepted": false
        }
    });

    // first check that response from deleting
    // crate_owner_invitation is okay
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("api/v1/me/crate_owner_invitations/{}", krate.id))
                .with_method(Method::Put)
                .with_body(body.to_string().as_bytes()),
        )
    );

    let json: T = ::json(&mut response);
    assert_eq!(json.crate_owner_invitation.accepted, false);
    assert_eq!(json.crate_owner_invitation.crate_id, krate.id);

    // then check to make sure that decline_invite did what it
    // was supposed to
    // crate_owner_invitation was deleted
    let mut response = ok_resp!(
        middle.call(
            req.with_path("api/v1/me/crate_owner_invitations")
                .with_method(Method::Get)
        )
    );
    let json: R = ::json(&mut response);
    assert_eq!(json.crate_owner_invitations.len(), 0);

    // new crate owner was not inserted
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/invited_crate/owners")
                .with_method(Method::Get)
        )
    );
    let json: Q = ::json(&mut response);
    assert_eq!(json.users.len(), 1);
}
