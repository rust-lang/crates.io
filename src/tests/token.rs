use crate::util::MockRequestExt;
use crate::{RequestHelper, TestApp};
use crates_io::{models::ApiToken, util::errors::TOKEN_FORMAT_ERROR, views::EncodableMe};
use diesel::prelude::*;
use http::{header, StatusCode};

#[tokio::test(flavor = "multi_thread")]
async fn using_token_updates_last_used_at() {
    let url = "/api/v1/me";
    let (app, anon, user, token) = TestApp::init().with_token();

    anon.get(url).await.assert_forbidden();
    user.get::<EncodableMe>(url).await.good();
    assert_none!(token.as_model().last_used_at);

    // Use the token once
    token.search("following=1").await;

    let token: ApiToken = app.db(|conn| {
        assert_ok!(ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .first(conn))
    });
    assert_some!(token.last_used_at);

    // Would check that it updates the timestamp here, but the timestamp is
    // based on the start of the database transaction so it doesn't work in
    // this test framework.
}

#[tokio::test(flavor = "multi_thread")]
async fn old_tokens_give_specific_error_message() {
    let url = "/api/v1/me";
    let (_, anon) = TestApp::init().empty();

    let mut request = anon.get_request(url);
    request.header(header::AUTHORIZATION, "oldtoken");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": TOKEN_FORMAT_ERROR }] })
    );
}
