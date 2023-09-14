use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::versions_published_by;
use crates_io::views::GoodCrate;
use diesel::{QueryDsl, RunQueryDsl};
use http::StatusCode;

#[test]
fn new_krate() {
    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_new", "1.0.0");
    let json: GoodCrate = user.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");

    let crates = app.crates_from_index_head("foo_new");
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "foo_new");
    assert_eq!(crates[0].vers, "1.0.0");
    assert!(crates[0].deps.is_empty());
    assert_eq!(
        crates[0].cksum,
        "8a8d84b87f379d5e32566b14df153c0ab0e1ea87dae79a00b891bb41f93dbbf6"
    );

    let expected_files = vec!["crates/foo_new/foo_new-1.0.0.crate", "index/fo/o_/foo_new"];
    assert_eq!(app.stored_files(), expected_files);

    app.db(|conn| {
        let email: String = versions_published_by::table
            .select(versions_published_by::email)
            .first(conn)
            .unwrap();
        assert_eq!(email, "something@example.com");
    });
}

#[test]
fn new_krate_with_token() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_new", "1.0.0");
    let json: GoodCrate = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");

    let expected_files = vec!["crates/foo_new/foo_new-1.0.0.crate", "index/fo/o_/foo_new"];
    assert_eq!(app.stored_files(), expected_files);
}

#[test]
fn new_krate_weird_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_weird", "0.0.0-pre");
    let json: GoodCrate = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_weird");
    assert_eq!(json.krate.max_version, "0.0.0-pre");

    let expected_files = vec![
        "crates/foo_weird/foo_weird-0.0.0-pre.crate",
        "index/fo/o_/foo_weird",
    ];
    assert_eq!(app.stored_files(), expected_files);
}

#[test]
fn new_krate_twice() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_twice", "0.99.0");
    token.publish_crate(crate_to_publish).good();

    let crate_to_publish =
        PublishBuilder::new("foo_twice", "2.0.0").description("2.0.0 description");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_twice");
    assert_eq!(json.krate.description.unwrap(), "2.0.0 description");

    let crates = app.crates_from_index_head("foo_twice");
    assert_eq!(crates.len(), 2);
    assert_eq!(crates[0].name, "foo_twice");
    assert_eq!(crates[0].vers, "0.99.0");
    assert!(crates[0].deps.is_empty());
    assert_eq!(crates[1].name, "foo_twice");
    assert_eq!(crates[1].vers, "2.0.0");
    assert!(crates[1].deps.is_empty());

    let expected_files = vec![
        "crates/foo_twice/foo_twice-0.99.0.crate",
        "crates/foo_twice/foo_twice-2.0.0.crate",
        "index/fo/o_/foo_twice",
    ];
    assert_eq!(app.stored_files(), expected_files);
}

#[test]
fn new_krate_duplicate_version() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database and then we'll try to publish the same version
        CrateBuilder::new("foo_dupe", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_dupe", "1.0.0");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "crate version `1.0.0` is already uploaded" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn empty_payload() {
    let (app, _, user) = TestApp::full().with_user();

    let response = user.put::<()>("/api/v1/crates/new", &[]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid metadata length" }] })
    );

    assert!(app.stored_files().is_empty());
}
