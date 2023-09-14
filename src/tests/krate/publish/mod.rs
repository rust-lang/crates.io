use crate::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::controllers::krate::publish::missing_metadata_error_message;
use crates_io::models::krate::MAX_NAME_LENGTH;
use crates_io::schema::versions_published_by;
use crates_io::views::GoodCrate;
use diesel::{QueryDsl, RunQueryDsl};
use http::StatusCode;
use std::collections::BTreeMap;
use std::iter::FromIterator;

mod audit_action;
mod auth;
mod build_metadata;
mod categories;
mod dependencies;
mod emails;
mod git;
mod inheritance;
mod keywords;
mod manifest;
mod max_size;
mod rate_limit;
mod readme;
mod similar_names;
mod tarball;

#[test]
fn uploading_new_version_touches_crate() {
    use crate::builders::PublishBuilder;
    use crate::util::{RequestHelper, TestApp};
    use crate::CrateResponse;
    use crates_io::schema::crates;
    use diesel::dsl::*;
    use diesel::{ExpressionMethods, RunQueryDsl};

    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at", "1.0.0");
    user.publish_crate(crate_to_publish).good();

    app.db(|conn| {
        diesel::update(crates::table)
            .set(crates::updated_at.eq(crates::updated_at - 1.hour()))
            .execute(conn)
            .unwrap();
    });

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_before = json.krate.updated_at;

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at", "2.0.0");
    user.publish_crate(crate_to_publish).good();

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_after = json.krate.updated_at;

    assert_ne!(updated_at_before, updated_at_after);
}

#[test]
fn invalid_names() {
    let (app, _, _, token) = TestApp::full().with_token();

    let bad_name = |name: &str, error_message: &str| {
        let crate_to_publish = PublishBuilder::new(name, "1.0.0");
        let response = token.publish_crate(crate_to_publish);
        assert_eq!(response.status(), StatusCode::OK);

        let json = response.into_json();
        let json = json.as_object().unwrap();
        let errors = json.get("errors").unwrap().as_array().unwrap();
        let first_error = errors.first().unwrap().as_object().unwrap();
        let detail = first_error.get("detail").unwrap().as_str().unwrap();
        assert!(detail.contains(error_message), "{detail:?}");
    };

    let error_message = "expected a valid crate name";
    bad_name("", error_message);
    bad_name("foo bar", error_message);
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1), error_message);
    bad_name("snow☃", error_message);
    bad_name("áccênts", error_message);

    let error_message = "cannot upload a crate with a reserved name";
    bad_name("std", error_message);
    bad_name("STD", error_message);
    bad_name("compiler-rt", error_message);
    bad_name("compiler_rt", error_message);
    bad_name("coMpiLer_Rt", error_message);

    assert!(app.stored_files().is_empty());
}

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
fn license_and_description_required() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description", "license"]) }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0").unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description"]) }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .license_file("foo")
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description"]) }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn features_version_2() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that foo_new can depend on it
        CrateBuilder::new("bar", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("bar");

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0")
        .dependency(dependency)
        .feature("new_feat", &["dep:bar", "bar?/feat"])
        .feature("old_feat", &[]);
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("foo");
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "foo");
    assert_eq!(crates[0].deps.len(), 1);
    assert_eq!(crates[0].v, Some(2));
    let features = BTreeMap::from_iter([("old_feat".to_string(), vec![])]);
    assert_eq!(crates[0].features, features);
    let features2 = BTreeMap::from_iter([(
        "new_feat".to_string(),
        vec!["dep:bar".to_string(), "bar?/feat".to_string()],
    )]);
    assert_eq!(crates[0].features2, Some(features2));
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
