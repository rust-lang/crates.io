use crate::tests::util::MockRequestExt;
use crate::tests::{RequestHelper, TestApp};
use crate::{models::ApiToken, views::EncodableMe};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::{header, StatusCode};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn using_token_updates_last_used_at() {
    let url = "/api/v1/me";
    let (app, anon, user, token) = TestApp::init().with_token().await;
    let mut conn = app.async_db_conn().await;

    anon.get(url).await.assert_forbidden();
    user.get::<EncodableMe>(url).await.good();
    assert_none!(token.as_model().last_used_at);

    // Use the token once
    token.search("following=1").await;

    let token: ApiToken = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .first(&mut conn)
            .await
    );
    assert_some!(token.last_used_at);

    // Would check that it updates the timestamp here, but the timestamp is
    // based on the start of the database transaction so it doesn't work in
    // this test framework.
}

#[tokio::test(flavor = "multi_thread")]
async fn old_tokens_give_specific_error_message() {
    let url = "/api/v1/me";
    let (_, anon) = TestApp::init().empty().await;

    let mut request = anon.get_request(url);
    request.header(header::AUTHORIZATION, "oldtoken");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The given API token does not match the format used by crates.io. Tokens generated before 2020-07-14 were generated with an insecure random number generator, and have been revoked. You can generate a new token at https://crates.io/me. For more information please see https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. We apologize for any inconvenience."}]}"#);
}
