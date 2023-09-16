use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn good_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish = PublishBuilder::new("foo_good_key", "1.0.0")
        .keyword("c++")
        .keyword("crates-io_index")
        .keyword("1password");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.name, "foo_good_key");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn bad_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish =
        PublishBuilder::new("foo_bad_key", "1.0.0").keyword("super-long-keyword-name-oh-no");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid length 29, expected a keyword with less than 20 characters at line 1 column 203" }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_bad_key", "1.0.0").keyword("?@?%");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid value: string \"?@?%\", expected a valid keyword specifier at line 1 column 178" }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_bad_key", "1.0.0").keyword("áccênts");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid value: string \"áccênts\", expected a valid keyword specifier at line 1 column 183" }] })
    );
}

#[test]
fn too_many_keywords() {
    let (app, _, _, token) = TestApp::full().with_token();
    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0")
            .keyword("one")
            .keyword("two")
            .keyword("three")
            .keyword("four")
            .keyword("five")
            .keyword("six"),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}
