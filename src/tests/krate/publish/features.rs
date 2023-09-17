use crate::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;
use std::collections::BTreeMap;

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
fn invalid_feature_name() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("~foo", &[]);
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}

#[test]
fn invalid_feature() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("foo", &["!bar"]);
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}
