use std::sync::ONCE_INIT;
use conduit::{Handler, Method};

use cargo_registry::user::NewUser;
use cargo_registry::krate::EncodableCrate;
use record::GhUser;

// Users: `crates-tester-1` and `crates-tester-2`
// Passwords: ask acrichto or gankro
// Teams: `crates-test-org:owners`, `crates-test-org:just-for-crates-2`
// tester-1 is on owners only, tester-2 is on both

static GH_USER_1: GhUser = GhUser {
    login: "crates-tester-1",
    init: ONCE_INIT,
};
static GH_USER_2: GhUser = GhUser {
    login: "crates-tester-2",
    init: ONCE_INIT,
};

fn mock_user_on_only_x() -> NewUser<'static> {
    GH_USER_1.user()
}
fn mock_user_on_x_and_y() -> NewUser<'static> {
    GH_USER_2.user()
}

fn body_for_team_y() -> &'static str {
    r#"{"users":["github:crates-test-org:just-for-crates-2"]}"#
}

fn body_for_team_x() -> &'static str {
    r#"{"users":["github:crates-test-org:owners"]}"#
}

// Test adding team without `github:`
#[test]
fn not_github() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_not_github");

    let body = r#"{"users":["dropbox:foo:foo"]}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_not_github/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains("unknown organization"),
        "{:?}",
        json.errors
    );
}

#[test]
fn weird_name() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_weird_name");

    let body = r#"{"users":["github:foo/../bar:wut"]}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_weird_name/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains(
            "organization cannot contain",
        ),
        "{:?}",
        json.errors
    );
}

// Test adding team without second `:`
#[test]
fn one_colon() {
    let (_b, app, middle) = ::app();
    let mut req = ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_one_colon");

    let body = r#"{"users":["github:foo"]}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_one_colon/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains("missing github team"),
        "{:?}",
        json.errors
    );
}

#[test]
fn nonexistent_team() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_nonexistent");

    let body = r#"{"users":["github:crates-test-org:this-does-not-exist"]}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_nonexistent/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains(
            "don't have permission to query a necessary property",
        ),
        "{:?}",
        json.errors
    );
}

// Test adding team as owner when on it
#[test]
fn add_team_as_member() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_team_member");

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_team_member/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
}

// Test adding team as owner when not on in
#[test]
fn add_team_as_non_member() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_only_x(), "foo_team_non_member");

    let body = body_for_team_y();
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_team_non_member/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains("don't have permission"),
        "{:?}",
        json.errors
    );
}

#[test]
fn remove_team_as_named_owner() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_remove_team");

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_remove_team/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_remove_team/owners")
                .with_method(Method::Delete)
                .with_body(body.as_bytes()),
        )
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = mock_user_on_only_x().create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }
    let body = ::new_req_body_version_2(::krate("foo_remove_team"));
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_body(&body)
                .with_method(Method::Put),
        )
    );
    assert!(
        json.errors[0].detail.contains("another user"),
        "{:?}",
        json.errors
    );
}

#[test]
fn remove_team_as_team_owner() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_remove_team_owner");

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_remove_team_owner/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = mock_user_on_only_x().create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }
    let body = body_for_team_x();
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_remove_team_owner/owners")
                .with_method(Method::Delete)
                .with_body(body.as_bytes()),
        )
    );

    assert!(
        json.errors[0].detail.contains("only owners have permission"),
        "{:?}",
        json.errors
    );

    let body = ::new_req_body_version_2(::krate("foo_remove_team_owner"));
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_body(&body)
                .with_method(Method::Put),
        )
    );
}

// Test trying to publish a krate we don't own
#[test]
fn publish_not_owned() {
    let (_b, app, middle) = ::app();

    let mut req = ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_not_owned");

    let body = body_for_team_y();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_not_owned/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = mock_user_on_only_x().create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }
    let body = ::new_req_body_version_2(::krate("foo_not_owned"));
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_body(&body)
                .with_method(Method::Put),
        )
    );
    assert!(
        json.errors[0].detail.contains("another user"),
        "{:?}",
        json.errors
    );
}

// Test trying to publish a krate we do own (but only because of teams)
#[test]
fn publish_owned() {
    let (_b, app, middle) = ::app();
    let mut req =
        ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_team_owned");

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_team_owned/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = mock_user_on_only_x().create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }
    let body = ::new_req_body_version_2(::krate("foo_team_owned"));
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_body(&body)
                .with_method(Method::Put),
        )
    );
}

// Test trying to change owners (when only on an owning team)
#[test]
fn add_owners_as_team_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::request_with_user_and_mock_crate(&app, mock_user_on_x_and_y(), "foo_add_owner");

    let body = body_for_team_x();
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_add_owner/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = mock_user_on_only_x().create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }
    let body = r#"{"users":["FlashCat"]}"#; // User doesn't matter
    let json = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_add_owner/owners")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(
        json.errors[0].detail.contains("only owners have permission"),
        "{:?}",
        json.errors
    );
}

#[test]
fn crates_by_team_id() {
    let (_b, app, middle) = ::app();

    let team = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("user_foo").create_or_update(&conn).unwrap();
        let t = ::new_team("team_foo").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("foo", u.id).expect_build(&conn);
        ::add_team_to_crate(&t, &krate, &u, &conn).unwrap();
        t
    };

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    req.with_query(&format!("team_id={}", team.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(Deserialize)]
    struct Response {
        crates: Vec<EncodableCrate>,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.crates.len(), 1);
}

#[test]
fn crates_by_team_id_not_including_deleted_owners() {
    let (_b, app, middle) = ::app();

    let team = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("user_foo").create_or_update(&conn).unwrap();
        let t = ::new_team("github:org_foo:team_foo")
            .create_or_update(&conn)
            .unwrap();
        let krate = ::CrateBuilder::new("foo", u.id).expect_build(&conn);
        ::add_team_to_crate(&t, &krate, &u, &conn).unwrap();
        krate.owner_remove(&conn, &u, &t.login).unwrap();
        t
    };

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    req.with_query(&format!("team_id={}", team.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(Deserialize)]
    struct Response {
        crates: Vec<EncodableCrate>,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.crates.len(), 0);
}
