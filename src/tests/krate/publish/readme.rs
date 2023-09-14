use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};

#[test]
fn new_krate_with_readme() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0").readme("hello world");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0");

    let expected_files = vec![
        "crates/foo_readme/foo_readme-1.0.0.crate",
        "index/fo/o_/foo_readme",
        "readmes/foo_readme/foo_readme-1.0.0.html",
    ];
    assert_eq!(app.stored_files(), expected_files);
}

#[test]
fn new_krate_with_empty_readme() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0").readme("");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0");

    let expected_files = vec![
        "crates/foo_readme/foo_readme-1.0.0.crate",
        "index/fo/o_/foo_readme",
    ];
    assert_eq!(app.stored_files(), expected_files);
}

#[test]
fn new_krate_with_readme_and_plus_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0+foo").readme("hello world");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0+foo");

    let expected_files = vec![
        "crates/foo_readme/foo_readme-1.0.0+foo.crate",
        "index/fo/o_/foo_readme",
        "readmes/foo_readme/foo_readme-1.0.0+foo.html",
    ];
    assert_eq!(app.stored_files(), expected_files);
}
