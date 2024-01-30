use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_display_snapshot;

#[test]
fn test_owner_change_with_invalid_json() {
    let (app, _, user) = TestApp::full().with_user();
    app.db_new_user("bar");
    app.db(|conn| CrateBuilder::new("foo", user.as_model().id).expect_build(conn));

    // incomplete input
    let input = r#"{"owners": ["foo", }"#;
    let response = user.delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes());
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_display_snapshot!(response.text(), @r###"{"errors":[{"detail":"Failed to parse the request body as JSON: owners[1]: expected value at line 1 column 20"}]}"###);

    // `owners` is not an array
    let input = r#"{"owners": "foo"}"#;
    let response = user.delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes());
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_display_snapshot!(response.text(), @r###"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: owners: invalid type: string \"foo\", expected a sequence at line 1 column 16"}]}"###);

    // missing `owners` and/or `users` fields
    let input = r#"{}"#;
    let response = user.delete_with_body::<()>("/api/v1/crates/foo/owners", input.as_bytes());
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_display_snapshot!(response.text(), @r###"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `owners` at line 1 column 2"}]}"###);
}

#[test]
fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user();
    app.db_new_user("bar");

    let body = json!({ "owners": ["bar"] });
    let body = serde_json::to_vec(&body).unwrap();

    let response = user.delete_with_body::<()>("/api/v1/crates/unknown/owners", body);
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_display_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown` does not exist"}]}"###);
}
