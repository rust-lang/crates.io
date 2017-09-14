use {CrateList, GoodCrate};

use cargo_registry::owner::EncodableOwner;
use cargo_registry::user::EncodablePublicUser;
use cargo_registry::crate_owner_invitation::{EncodableCrateOwnerInvitation,
                                             NewCrateOwnerInvitation};
use cargo_registry::schema::crate_owner_invitations;

use conduit::{Handler, Method};
use diesel;
use diesel::prelude::*;

#[derive(Deserialize)]
struct TeamResponse {
    teams: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
struct UserResponse {
    users: Vec<EncodableOwner>,
}

#[test]
#[ignore]
fn new_crate_owner() {
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }

    let (_b, app, middle) = ::app();

    // Create a crate under one user
    let mut req = ::new_req(app.clone(), "foo_owner", "1.0.0");
    ::sign_in(&mut req, &app);
    let u2;
    {
        let conn = app.diesel_database.get().unwrap();
        u2 = ::new_user("bar").create_or_update(&conn).unwrap();
    }
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    // Flag the second user as an owner
    let body = r#"{"users":["bar"]}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_owner/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<O>(&mut response).ok);
    bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_owner/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    // Make sure this shows up as one of their crates.
    let query = format!("user_id={}", u2.id);
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates")
                .with_method(Method::Get)
                .with_query(&query),
        )
    );
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);

    // And upload a new crate as the first user
    let body = ::new_req_body_version_2(::krate("foo_owner"));
    ::sign_in_as(&mut req, &u2);
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_method(Method::Put)
                .with_body(&body),
        )
    );
    ::json::<GoodCrate>(&mut response);
}

// Ensures that so long as at least one owner remains associated with the crate,
// a user can still remove their own login as an owner
#[test]
fn owners_can_remove_self() {
    #[derive(Deserialize)]
    struct R {
        users: Vec<EncodablePublicUser>,
    }
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/owners_selfremove/owners",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        ::new_user("secondowner").create_or_update(&conn).unwrap();
        let user = ::new_user("firstowner").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("owners_selfremove", user.id).expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    // Deleting yourself when you're the only owner isn't allowed.
    let body = r#"{"users":["firstowner"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete).with_body(
        body.as_bytes(),
    )));
    let json = ::json::<::Bad>(&mut response);
    assert!(json.errors[0].detail.contains(
        "cannot remove the sole owner of a crate",
    ));

    let body = r#"{"users":["secondowner"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Put).with_body(
        body.as_bytes(),
    )));
    assert!(::json::<O>(&mut response).ok);

    // Deleting yourself when there are other owners is allowed.
    let body = r#"{"users":["firstowner"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete).with_body(
        body.as_bytes(),
    )));
    assert!(::json::<O>(&mut response).ok);

    // After you delete yourself, you no longer have permisions to manage the crate.
    let body = r#"{"users":["secondowner"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete).with_body(
        body.as_bytes(),
    )));
    let json = ::json::<::Bad>(&mut response);
    assert!(json.errors[0].detail.contains(
        "only owners have permission to modify owners",
    ));
}

#[test]
fn owners() {
    #[derive(Deserialize)]
    struct R {
        users: Vec<EncodablePublicUser>,
    }
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates/foo_owners/owners");
    {
        let conn = app.diesel_database.get().unwrap();
        ::new_user("foobar").create_or_update(&conn).unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_owners", user.id).expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Put).with_body(
        body.as_bytes(),
    )));
    assert!(::json::<O>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 2);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete).with_body(
        body.as_bytes(),
    )));
    assert!(::json::<O>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let body = r#"{"users":["foo"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete).with_body(
        body.as_bytes(),
    )));
    ::json::<::Bad>(&mut response);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Put).with_body(
        body.as_bytes(),
    )));
    assert!(::json::<O>(&mut response).ok);
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
    let (_b, app, middle) = ::app();

    let (krate_owned_by_team, team) = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("user_foo").create_or_update(&conn).unwrap();
        let t = ::new_team("team_foo").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("foo", u.id).expect_build(&conn);
        ::add_team_to_crate(&t, &krate, &u, &conn).unwrap();
        (krate, t)
    };

    let (krate_not_owned_by_team, user) = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("user_bar").create_or_update(&conn).unwrap();
        (::CrateBuilder::new("bar", u.id).expect_build(&conn), u)
    };

    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates");

    let query = format!("user_id={}", user.id);
    let mut response = ok_resp!(middle.call(req.with_query(&query)));
    let json: CrateList = ::json(&mut response);

    assert_eq!(json.crates[0].name, krate_not_owned_by_team.name);
    assert_eq!(json.crates.len(), 1);

    let query = format!("team_id={}", team.id);
    let mut response = ok_resp!(middle.call(req.with_query(&query)));

    let json: CrateList = ::json(&mut response);
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
    let (_b, app, middle) = ::app();

    let (team, user) = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("user_cat").create_or_update(&conn).unwrap();
        let t = ::new_team("github:test_org:team_sloth")
            .create_or_update(&conn)
            .unwrap();
        let krate = ::CrateBuilder::new("best_crate", u.id).expect_build(&conn);
        ::add_team_to_crate(&t, &krate, &u, &conn).unwrap();
        (t, u)
    };

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/best_crate/owner_team",
    );
    let mut response = ok_resp!(middle.call(&mut req));
    let json: TeamResponse = ::json(&mut response);

    assert_eq!(json.teams[0].kind, "team");
    assert_eq!(json.teams[0].name, team.name);

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/best_crate/owner_user",
    );
    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserResponse = ::json(&mut response);

    assert_eq!(json.users[0].kind, "user");
    assert_eq!(json.users[0].name, user.name);
}

#[test]
fn invitations_are_empty_by_default() {
    #[derive(Deserialize)]
    struct R {
        invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/me/crate_owner_invitations",
    );

    let user = {
        let conn = app.diesel_database.get().unwrap();
        ::new_user("user_no_invites")
            .create_or_update(&conn)
            .unwrap()
    };
    ::sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: R = ::json(&mut response);

    assert_eq!(json.invitations.len(), 0);
}

#[test]
fn invitations_list() {
    #[derive(Deserialize)]
    struct R {
        invitations: Vec<EncodableCrateOwnerInvitation>,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/me/crate_owner_invitations",
    );
    let (krate, user) = {
        let conn = app.diesel_database.get().unwrap();
        let owner = ::new_user("inviting_user").create_or_update(&conn).unwrap();
        let user = ::new_user("invited_user").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("invited_crate", owner.id).expect_build(&conn);

        // This should be replaced by an actual call to the route that `owner --add` hits once
        // that route creates an invitation.
        let invitation = NewCrateOwnerInvitation {
            invited_by_user_id: owner.id,
            invited_user_id: user.id,
            crate_id: krate.id,
        };
        diesel::insert(&invitation)
            .into(crate_owner_invitations::table)
            .execute(&*conn)
            .unwrap();
        (krate, user)
    };
    ::sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: R = ::json(&mut response);

    assert_eq!(json.invitations.len(), 1);
    assert_eq!(json.invitations[0].invited_by_username, "inviting_user");
    assert_eq!(json.invitations[0].crate_name, "invited_crate");
    assert_eq!(json.invitations[0].crate_id, krate.id);
}
