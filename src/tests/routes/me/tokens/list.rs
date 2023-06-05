use crate::util::insta::{self, assert_yaml_snapshot};
use crate::util::{RequestHelper, TestApp};
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::models::ApiToken;
use http::StatusCode;

#[test]
fn list_logged_out() {
    let (_, anon) = TestApp::init().empty();
    anon.get("/api/v1/me/tokens").assert_forbidden();
}

#[test]
fn list_with_api_token_is_forbidden() {
    let (_, _, _, token) = TestApp::init().with_token();
    token.get("/api/v1/me/tokens").assert_forbidden();
}

#[test]
fn list_empty() {
    let (_, _, user) = TestApp::init().with_user();
    let response = user.get::<()>("/api/v1/me/tokens");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.into_json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 0);
}

#[test]
fn list_tokens() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    app.db(|conn| {
        vec![
            assert_ok!(ApiToken::insert(conn, id, "bar")),
            assert_ok!(ApiToken::insert_with_scopes(
                conn,
                id,
                "baz",
                Some(vec![
                    CrateScope::try_from("serde").unwrap(),
                    CrateScope::try_from("serde-*").unwrap()
                ]),
                Some(vec![EndpointScope::PublishUpdate])
            )),
        ]
    });

    let response = user.get::<()>("/api/v1/me/tokens");
    assert_eq!(response.status(), StatusCode::OK);
    assert_yaml_snapshot!(response.into_json(), {
        ".api_tokens[].id" => insta::any_id_redaction(),
        ".api_tokens[].created_at" => "[datetime]",
        ".api_tokens[].last_used_at" => "[datetime]",
    });
}

#[test]
fn list_tokens_exclude_revoked() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    let tokens = app.db(|conn| {
        vec![
            assert_ok!(ApiToken::insert(conn, id, "bar")),
            assert_ok!(ApiToken::insert(conn, id, "baz")),
        ]
    });

    // List tokens expecting them all to be there.
    let response = user.get::<()>("/api/v1/me/tokens");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.into_json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 2);

    // Revoke the first token.
    let response = user.delete::<()>(&format!("/api/v1/me/tokens/{}", tokens[0].model.id));
    assert_eq!(response.status(), StatusCode::OK);

    // Check that we now have one less token being listed.
    let response = user.get::<()>("/api/v1/me/tokens");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.into_json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 1);
    assert_eq!(response_tokens[0]["name"], json!("baz"));
}
