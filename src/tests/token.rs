use crate::{user::UserShowPrivateResponse, RequestHelper, TestApp};
use cargo_registry::{
    models::ApiToken,
    schema::api_tokens,
    views::{EncodableApiTokenWithToken, EncodableMe},
};
use std::collections::HashSet;

use diesel::prelude::*;

#[derive(Deserialize)]
struct DecodableApiToken {
    name: String,
}

#[derive(Deserialize)]
struct ListResponse {
    api_tokens: Vec<DecodableApiToken>,
}
#[derive(Deserialize)]
struct NewResponse {
    api_token: EncodableApiTokenWithToken,
}
#[derive(Deserialize)]
struct RevokedResponse {}

macro_rules! assert_contains {
    ($e:expr, $f:expr) => {
        if !$e.contains($f) {
            panic!(format!("expected '{}' to contain '{}'", $e, $f));
        }
    };
}

// Default values used by many tests
static URL: &str = "/api/v1/me/tokens";
static NEW_BAR: &[u8] = br#"{ "api_token": { "name": "bar" } }"#;

#[test]
fn list_logged_out() {
    let (_, anon) = TestApp::init().empty();
    anon.get(URL).assert_forbidden();
}

#[test]
fn list_empty() {
    let (_, _, user) = TestApp::init().with_user();
    let json: ListResponse = user.get(URL).good();
    assert_eq!(json.api_tokens.len(), 0);
}

#[test]
fn list_tokens() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    let tokens = app.db(|conn| {
        vec![
            t!(ApiToken::insert(conn, id, "bar")),
            t!(ApiToken::insert(conn, id, "baz")),
        ]
    });

    let json: ListResponse = user.get(URL).good();
    assert_eq!(json.api_tokens.len(), tokens.len());
    assert_eq!(
        json.api_tokens
            .into_iter()
            .map(|t| t.name)
            .collect::<HashSet<_>>(),
        tokens.into_iter().map(|t| t.name).collect::<HashSet<_>>()
    );
}

#[test]
fn list_tokens_exclude_revoked() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    let tokens = app.db(|conn| {
        vec![
            t!(ApiToken::insert(conn, id, "bar")),
            t!(ApiToken::insert(conn, id, "baz")),
        ]
    });

    // List tokens expecting them all to be there.
    let json: ListResponse = user.get(URL).good();
    assert_eq!(json.api_tokens.len(), tokens.len());

    // Revoke the first token.
    let _json: RevokedResponse = user
        .delete(&format!("/api/v1/me/tokens/{}", tokens[0].id))
        .good();

    // Check that we now have one less token being listed.
    let json: ListResponse = user.get(URL).good();
    assert_eq!(json.api_tokens.len(), tokens.len() - 1);
    assert!(json
        .api_tokens
        .iter()
        .find(|token| token.name == tokens[0].name)
        .is_none());
}

#[test]
fn create_token_logged_out() {
    let (_, anon) = TestApp::init().empty();
    anon.put(URL, NEW_BAR).assert_forbidden();
}

#[test]
fn create_token_invalid_request() {
    let (_, _, user) = TestApp::init().with_user();
    let invalid = br#"{ "name": "" }"#;
    let json = user.put::<()>(URL, invalid).bad_with_status(400);

    assert_contains!(json.errors[0].detail, "invalid new token request");
}

#[test]
fn create_token_no_name() {
    let (_, _, user) = TestApp::init().with_user();
    let empty_name = br#"{ "api_token": { "name": "" } }"#;
    let json = user.put::<()>(URL, empty_name).bad_with_status(400);

    assert_eq!(json.errors[0].detail, "name must have a value");
}

#[test]
fn create_token_long_body() {
    let (_, _, user) = TestApp::init().with_user();
    let too_big = &[5; 5192]; // Send a request with a 5kB body of 5's
    let json = user.put::<()>(URL, too_big).bad_with_status(400);

    assert_contains!(json.errors[0].detail, "max content length");
}

#[test]
fn create_token_exceeded_tokens_per_user() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    app.db(|conn| {
        for i in 0..1000 {
            t!(ApiToken::insert(conn, id, &format!("token {}", i)));
        }
    });
    let json = user.put::<()>(URL, NEW_BAR).bad_with_status(400);

    assert_contains!(json.errors[0].detail, "maximum tokens per user");
}

#[test]
fn create_token_success() {
    let (app, _, user) = TestApp::init().with_user();

    let json: NewResponse = user.put(URL, NEW_BAR).good();
    assert_eq!(json.api_token.name, "bar");
    assert!(!json.api_token.token.is_empty());

    let tokens = app.db(|conn| t!(ApiToken::belonging_to(user.as_model()).load::<ApiToken>(conn)));
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, "bar");
    assert_eq!(tokens[0].token, json.api_token.token);
    assert_eq!(tokens[0].revoked, false);
    assert_eq!(tokens[0].last_used_at, None);
}

#[test]
fn create_token_multiple_have_different_values() {
    let (_, _, user) = TestApp::init().with_user();
    let first: NewResponse = user.put(URL, NEW_BAR).good();
    let second: NewResponse = user.put(URL, NEW_BAR).good();

    assert_ne!(first.api_token.token, second.api_token.token);
}

#[test]
fn create_token_multiple_users_have_different_values() {
    let (app, _, user1) = TestApp::init().with_user();
    let first_token: NewResponse = user1.put(URL, NEW_BAR).good();

    let user2 = app.db_new_user("bar");
    let second_token: NewResponse = user2.put(URL, NEW_BAR).good();

    assert_ne!(first_token.api_token.token, second_token.api_token.token);
}

#[test]
fn cannot_create_token_with_token() {
    let (_, _, _, token) = TestApp::init().with_token();
    let json = token
        .put::<()>(
            "/api/v1/me/tokens",
            br#"{ "api_token": { "name": "baz" } }"#,
        )
        .bad_with_status(400);

    assert_contains!(
        json.errors[0].detail,
        "cannot use an API token to create a new API token"
    );
}

#[test]
fn revoke_token_non_existing() {
    let (_, _, user) = TestApp::init().with_user();
    let _json: RevokedResponse = user.delete("/api/v1/me/tokens/5").good();
}

#[test]
fn revoke_token_doesnt_revoke_other_users_token() {
    let (app, _, user1, token) = TestApp::init().with_token();
    let user1 = user1.as_model();
    let token = token.as_model();
    let user2 = app.db_new_user("baz");

    // List tokens for first user contains the token
    app.db(|conn| {
        let tokens = t!(ApiToken::belonging_to(user1).load::<ApiToken>(conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.name);
    });

    // Try revoke the token as second user
    let _json: RevokedResponse = user2
        .delete(&format!("/api/v1/me/tokens/{}", token.id))
        .good();

    // List tokens for first user still contains the token
    app.db(|conn| {
        let tokens = t!(ApiToken::belonging_to(user1).load::<ApiToken>(conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.name);
    });
}

#[test]
fn revoke_token_success() {
    let (app, _, user, token) = TestApp::init().with_token();

    // List tokens contains the token
    app.db(|conn| {
        let tokens = t!(ApiToken::belonging_to(user.as_model()).load::<ApiToken>(conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.as_model().name);
    });

    // Revoke the token
    let _json: RevokedResponse = user
        .delete(&format!("/api/v1/me/tokens/{}", token.as_model().id))
        .good();

    // List tokens no longer contains the token
    app.db(|conn| {
        let count = ApiToken::belonging_to(user.as_model())
            .filter(api_tokens::revoked.eq(false))
            .count()
            .get_result(conn);
        assert_eq!(count, Ok(0));
    });
}

#[test]
fn token_gives_access_to_me() {
    let url = "/api/v1/me";
    let (_, anon, user, token) = TestApp::init().with_token();

    anon.get(url).assert_forbidden();

    let json: UserShowPrivateResponse = token.get(url).good();
    assert_eq!(json.user.email, user.as_model().email);
}

#[test]
fn using_token_updates_last_used_at() {
    let url = "/api/v1/me";
    let (app, anon, user, token) = TestApp::init().with_token();

    anon.get(url).assert_forbidden();
    user.get::<EncodableMe>(url).good();
    assert!(token.as_model().last_used_at.is_none());

    // Use the token once
    token.get::<EncodableMe>("/api/v1/me").good();

    let token = app.db(|conn| t!(ApiToken::belonging_to(user.as_model()).first::<ApiToken>(conn)));
    assert!(token.last_used_at.is_some());

    // Would check that it updates the timestamp here, but the timestamp is
    // based on the start of the database transaction so it doesn't work in
    // this test framework.
}
