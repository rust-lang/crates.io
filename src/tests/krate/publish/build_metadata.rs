use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

fn version_with_build_metadata(v1: &str, v2: &str, expected_error: &str) {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo", v1));
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let response = token.publish_crate(PublishBuilder::new("foo", v2));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": expected_error }] })
    );
}

#[test]
fn version_with_build_metadata_1() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_1" }, {
        version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0+bar",
            "crate version `1.0.0` is already uploaded",
        );
    });
}

#[test]
fn version_with_build_metadata_2() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_2" }, {
        version_with_build_metadata(
            "1.0.0-beta.1",
            "1.0.0-beta.1+2",
            "crate version `1.0.0-beta.1` is already uploaded",
        );
    });
}

#[test]
fn version_with_build_metadata_3() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_3" }, {
        version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0",
            "crate version `1.0.0` is already uploaded",
        );
    });
}
