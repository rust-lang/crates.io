use crate::models::ApiToken;
use crate::tests::util::MockRequestExt;
use crate::tests::{RequestHelper, TestApp};
use diesel::associations::HasTable;
use diesel::prelude::*;
use http::{header, StatusCode};
use insta::assert_snapshot;

fn get_token(conn: &mut PgConnection, id: i32) -> ApiToken {
    ApiToken::table()
        .find(id)
        .select(ApiToken::as_select())
        .first(conn)
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn using_token_updates_last_used_at() {
    let (app, _, _, token_client) = TestApp::init().with_token();

    let token_id = token_client.as_model().id;
    let token = app.db(|conn| get_token(conn, token_id));
    assert_none!(token.last_used_at);

    let response = token_client.get::<()>("/api/v1/crates?following=1").await;
    assert_eq!(response.status(), StatusCode::OK);

    let token = app.db(|conn| get_token(conn, token_id));
    let last_used_at1 = assert_some!(token.last_used_at);

    let response = token_client.get::<()>("/api/v1/crates?following=1").await;
    assert_eq!(response.status(), StatusCode::OK);

    let token = app.db(|conn| get_token(conn, token_id));
    let last_used_at2 = assert_some!(token.last_used_at);
    assert!(last_used_at2 > last_used_at1);
}

#[tokio::test(flavor = "multi_thread")]
async fn old_tokens_give_specific_error_message() {
    let url = "/api/v1/me";
    let (_, anon) = TestApp::init().empty();

    let mut request = anon.get_request(url);
    request.header(header::AUTHORIZATION, "oldtoken");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The given API token does not match the format used by crates.io. Tokens generated before 2020-07-14 were generated with an insecure random number generator, and have been revoked. You can generate a new token at https://crates.io/me. For more information please see https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. We apologize for any inconvenience."}]}"#);
}
