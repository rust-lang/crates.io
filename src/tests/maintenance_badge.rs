use std::collections::HashMap;

use cargo_registry::models::Badge;
use conduit::StatusCode;

use crate::util::{MockAnonymousUser, RequestHelper};
use crate::{builders::CrateBuilder, TestApp};

fn set_up() -> MockAnonymousUser {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let mut badges = HashMap::new();
        badges.insert("maintenance".to_owned(), {
            let mut attributes = HashMap::new();
            attributes.insert("status".to_owned(), "looking-for-maintainer".to_owned());
            attributes
        });

        let krate = CrateBuilder::new("foo", user.id).expect_build(conn);
        Badge::update_crate(conn, &krate, Some(&badges)).unwrap();

        CrateBuilder::new("bar", user.id).expect_build(conn);
    });

    anon
}

#[test]
fn crate_with_maintenance_badge() {
    let anon = set_up();

    let response = anon
        .get::<String>("/api/v1/crates/foo/maintenance.svg")
        .good_text();

    assert!(response.contains("looking-for-maintainer"));
    assert!(response.contains("fill=\"orange\""));
}

#[test]
fn crate_without_maintenance_badge() {
    let anon = set_up();

    let response = anon
        .get::<String>("/api/v1/crates/bar/maintenance.svg")
        .good_text();

    assert!(response.contains("unknown"));
    assert!(response.contains("fill=\"lightgrey\""));
}

#[test]
fn unknown_crate() {
    let anon = set_up();

    anon.get::<()>("/api/v1/crates/unknown/maintenance.svg")
        .assert_status(StatusCode::NOT_FOUND);
}
