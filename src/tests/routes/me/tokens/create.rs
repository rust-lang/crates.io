use crate::models::token::{CrateScope, EndpointScope};
use crate::models::ApiToken;
use crate::tests::util::insta::{self, assert_json_snapshot};
use crate::tests::util::{RequestHelper, TestApp};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;
use serde_json::Value;

static NEW_BAR: &[u8] = br#"{ "api_token": { "name": "bar" } }"#;

#[tokio::test(flavor = "multi_thread")]
async fn create_token_logged_out() {
    let (_, anon) = TestApp::init().empty();
    anon.put("/api/v1/me/tokens", NEW_BAR)
        .await
        .assert_forbidden();
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_invalid_request() {
    let (app, _, user) = TestApp::init().with_user();
    let invalid: &[u8] = br#"{ "name": "" }"#;
    let response = user.put::<()>("/api/v1/me/tokens", invalid).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `api_token` at line 1 column 14"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_no_name() {
    let (app, _, user) = TestApp::init().with_user();
    let empty_name: &[u8] = br#"{ "api_token": { "name": "" } }"#;
    let response = user.put::<()>("/api/v1/me/tokens", empty_name).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"name must have a value"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_exceeded_tokens_per_user() {
    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.db_conn();
    let id = user.as_model().id;

    for i in 0..1000 {
        assert_ok!(ApiToken::insert(&mut conn, id, &format!("token {i}")));
    }

    let response = user.put::<()>("/api/v1/me/tokens", NEW_BAR).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"maximum tokens per user is: 500"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_success() {
    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.async_db_conn().await;

    let response = user.put::<()>("/api/v1/me/tokens", NEW_BAR).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.id" => insta::any_id_redaction(),
        ".api_token.created_at" => "[datetime]",
        ".api_token.last_used_at" => "[datetime]",
        ".api_token.token" => insta::api_token_redaction(),
    });

    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );

    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, "bar");
    assert!(!tokens[0].revoked);
    assert_eq!(tokens[0].last_used_at, None);
    assert_eq!(tokens[0].crate_scopes, None);
    assert_eq!(tokens[0].endpoint_scopes, None);

    assert_snapshot!(app.emails_snapshot());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_multiple_have_different_values() {
    let (_, _, user) = TestApp::init().with_user();
    let first: Value = user.put("/api/v1/me/tokens", NEW_BAR).await.good();
    let second: Value = user.put("/api/v1/me/tokens", NEW_BAR).await.good();

    assert_eq!(first["api_token"]["name"], second["api_token"]["name"]);
    assert_ne!(first["api_token"]["token"], second["api_token"]["token"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_multiple_users_have_different_values() {
    let (app, _, user1) = TestApp::init().with_user();
    let first: Value = user1.put("/api/v1/me/tokens", NEW_BAR).await.good();

    let user2 = app.db_new_user("bar");
    let second: Value = user2.put("/api/v1/me/tokens", NEW_BAR).await.good();

    assert_ne!(first["api_token"]["token"], second["api_token"]["token"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_create_token_with_token() {
    let (app, _, _, token) = TestApp::init().with_token();
    let response = token
        .put::<()>(
            "/api/v1/me/tokens",
            br#"{ "api_token": { "name": "baz" } }"# as &[u8],
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot use an API token to create a new API token"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_with_scopes() {
    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.async_db_conn().await;

    let json = json!({
        "api_token": {
            "name": "bar",
            "crate_scopes": ["tokio", "tokio-*"],
            "endpoint_scopes": ["publish-update"],
        }
    });

    let response = user
        .put::<()>("/api/v1/me/tokens", serde_json::to_vec(&json).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.id" => insta::any_id_redaction(),
        ".api_token.created_at" => "[datetime]",
        ".api_token.last_used_at" => "[datetime]",
        ".api_token.token" => insta::api_token_redaction(),
    });

    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );

    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, "bar");
    assert!(!tokens[0].revoked);
    assert_eq!(tokens[0].last_used_at, None);
    assert_eq!(
        tokens[0].crate_scopes,
        Some(vec![
            CrateScope::try_from("tokio").unwrap(),
            CrateScope::try_from("tokio-*").unwrap()
        ])
    );
    assert_eq!(
        tokens[0].endpoint_scopes,
        Some(vec![EndpointScope::PublishUpdate])
    );

    assert_snapshot!(app.emails_snapshot());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_with_null_scopes() {
    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.async_db_conn().await;

    let json = json!({
        "api_token": {
            "name": "bar",
            "crate_scopes": null,
            "endpoint_scopes": null,
        }
    });

    let response = user
        .put::<()>("/api/v1/me/tokens", serde_json::to_vec(&json).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.id" => insta::any_id_redaction(),
        ".api_token.created_at" => "[datetime]",
        ".api_token.last_used_at" => "[datetime]",
        ".api_token.token" => insta::api_token_redaction(),
    });

    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );

    assert_that!(tokens, len(eq(1)));
    assert_eq!(tokens[0].name, "bar");
    assert!(!tokens[0].revoked);
    assert_eq!(tokens[0].last_used_at, None);
    assert_eq!(tokens[0].crate_scopes, None);
    assert_eq!(tokens[0].endpoint_scopes, None);

    assert_snapshot!(app.emails_snapshot());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_with_empty_crate_scope() {
    let (app, _, user) = TestApp::init().with_user();

    let json = json!({
        "api_token": {
            "name": "bar",
            "crate_scopes": ["", "tokio-*"],
            "endpoint_scopes": ["publish-update"],
        }
    });

    let response = user
        .put::<()>("/api/v1/me/tokens", serde_json::to_vec(&json).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid crate scope"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_with_invalid_endpoint_scope() {
    let (app, _, user) = TestApp::init().with_user();

    let json = json!({
        "api_token": {
            "name": "bar",
            "crate_scopes": ["tokio", "tokio-*"],
            "endpoint_scopes": ["crash"],
        }
    });

    let response = user
        .put::<()>("/api/v1/me/tokens", serde_json::to_vec(&json).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid endpoint scope"}]}"#);
    assert!(app.emails().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn create_token_with_expiry_date() {
    let (app, _, user) = TestApp::init().with_user();

    let json = json!({
        "api_token": {
            "name": "bar",
            "crate_scopes": null,
            "endpoint_scopes": null,
            "expired_at": "2024-12-24T12:34:56+05:00",
        }
    });

    let response = user
        .put::<()>("/api/v1/me/tokens", serde_json::to_vec(&json).unwrap())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.id" => insta::any_id_redaction(),
        ".api_token.created_at" => "[datetime]",
        ".api_token.last_used_at" => "[datetime]",
        ".api_token.token" => insta::api_token_redaction(),
    });

    assert_snapshot!(app.emails_snapshot());
}
