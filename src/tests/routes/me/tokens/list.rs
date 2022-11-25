use crate::routes::me::tokens::delete::RevokedResponse;
use crate::util::{RequestHelper, TestApp};
use cargo_registry::models::ApiToken;
use std::collections::HashSet;

#[derive(Deserialize)]
struct DecodableApiToken {
    name: String,
}

#[derive(Deserialize)]
struct ListResponse {
    api_tokens: Vec<DecodableApiToken>,
}

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
    let json: ListResponse = user.get("/api/v1/me/tokens").good();
    assert_eq!(json.api_tokens.len(), 0);
}

#[test]
fn list_tokens() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    let tokens = app.db(|conn| {
        vec![
            assert_ok!(ApiToken::insert(conn, id, "bar")),
            assert_ok!(ApiToken::insert(conn, id, "baz")),
        ]
    });

    let json: ListResponse = user.get("/api/v1/me/tokens").good();
    assert_eq!(json.api_tokens.len(), tokens.len());
    assert_eq!(
        json.api_tokens
            .into_iter()
            .map(|t| t.name)
            .collect::<HashSet<_>>(),
        tokens
            .into_iter()
            .map(|t| t.model.name)
            .collect::<HashSet<_>>()
    );
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
    let json: ListResponse = user.get("/api/v1/me/tokens").good();
    assert_eq!(json.api_tokens.len(), tokens.len());

    // Revoke the first token.
    let _json: RevokedResponse = user
        .delete(&format!("/api/v1/me/tokens/{}", tokens[0].model.id))
        .good();

    // Check that we now have one less token being listed.
    let json: ListResponse = user.get("/api/v1/me/tokens").good();
    assert_eq!(json.api_tokens.len(), tokens.len() - 1);
    assert!(!json
        .api_tokens
        .iter()
        .any(|token| token.name == tokens[0].model.name));
}
