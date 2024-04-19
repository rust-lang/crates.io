use crate::util::{RequestHelper, TestApp};
use crates_io::models::ApiToken;
use crates_io::schema::api_tokens;
use diesel::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

#[test]
fn revoke_current_token_success() {
    let (app, _, user, token) = TestApp::init().with_token();

    // Ensure that the token currently exists in the database
    app.db(|conn| {
        let tokens: Vec<ApiToken> = assert_ok!(ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.as_model().name);
    });

    // Revoke the token
    let response = token.delete::<()>("/api/v1/tokens/current");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Ensure that the token was removed from the database
    app.db(|conn| {
        let tokens: Vec<ApiToken> = assert_ok!(ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(conn));
        assert_eq!(tokens.len(), 0);
    });
}

#[test]
fn revoke_current_token_without_auth() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.delete::<()>("/api/v1/tokens/current");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);
}

#[test]
fn revoke_current_token_with_cookie_user() {
    let (app, _, user, token) = TestApp::init().with_token();

    // Ensure that the token currently exists in the database
    app.db(|conn| {
        let tokens: Vec<ApiToken> = assert_ok!(ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.as_model().name);
    });

    // Revoke the token
    let response = user.delete::<()>("/api/v1/tokens/current");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "token not provided" }] })
    );

    // Ensure that the token still exists in the database after the failed request
    app.db(|conn| {
        let tokens: Vec<ApiToken> = assert_ok!(ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .filter(api_tokens::revoked.eq(false))
            .load(conn));
        assert_eq!(tokens.len(), 1);
    });
}
