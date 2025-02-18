use crate::models::token::{CrateScope, EndpointScope, NewApiToken};
use crate::tests::util::insta::{self, assert_json_snapshot};
use crate::tests::util::{RequestHelper, TestApp};
use chrono::{Duration, Utc};
use http::StatusCode;
use insta::assert_snapshot;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn list_logged_out() {
    let (_, anon) = TestApp::init().empty().await;
    anon.get("/api/v1/me/tokens").await.assert_forbidden();
}

#[tokio::test(flavor = "multi_thread")]
async fn list_with_api_token_is_forbidden() {
    let (_, _, _, token) = TestApp::init().with_token().await;
    token.get("/api/v1/me/tokens").await.assert_forbidden();
}

#[tokio::test(flavor = "multi_thread")]
async fn list_empty() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/me/tokens").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"api_tokens":[]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn list_tokens() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let id = user.as_model().id;

    let new_token = NewApiToken::builder().name("bar").user_id(id).build();
    assert_ok!(new_token.insert(&mut conn).await);

    let new_token = NewApiToken::builder()
        .name("baz")
        .user_id(id)
        .crate_scopes(vec![
            CrateScope::try_from("serde").unwrap(),
            CrateScope::try_from("serde-*").unwrap(),
        ])
        .endpoint_scopes(vec![EndpointScope::PublishUpdate])
        .build();
    assert_ok!(new_token.insert(&mut conn).await);

    let new_token = NewApiToken::builder()
        .name("qux")
        .user_id(id)
        .expired_at(Utc::now() - Duration::days(1))
        .build();
    assert_ok!(new_token.insert(&mut conn).await);

    let response = user.get::<()>("/api/v1/me/tokens").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_tokens[].id" => insta::any_id_redaction(),
        ".api_tokens[].created_at" => "[datetime]",
        ".api_tokens[].last_used_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn list_recently_expired_tokens() {
    #[track_caller]
    fn assert_response_tokens_contain_name(response_tokens: &[serde_json::Value], name: &str) {
        assert_some!(response_tokens.iter().find(|token| token["name"] == name));
    }

    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let id = user.as_model().id;

    let new_token = NewApiToken::builder().name("bar").user_id(id).build();
    assert_ok!(new_token.insert(&mut conn).await);

    let new_token = NewApiToken::builder()
        .name("ancient")
        .user_id(id)
        .crate_scopes(vec![
            CrateScope::try_from("serde").unwrap(),
            CrateScope::try_from("serde-*").unwrap(),
        ])
        .endpoint_scopes(vec![EndpointScope::PublishUpdate])
        .expired_at(Utc::now() - Duration::days(31))
        .build();
    assert_ok!(new_token.insert(&mut conn).await);

    let new_token = NewApiToken::builder()
        .name("recent")
        .user_id(id)
        .expired_at(Utc::now() - Duration::days(1))
        .build();
    assert_ok!(new_token.insert(&mut conn).await);

    let response = user.get::<()>("/api/v1/me/tokens?expired_days=30").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 2);
    assert_response_tokens_contain_name(response_tokens, "bar");
    assert_response_tokens_contain_name(response_tokens, "recent");

    let response = user.get::<()>("/api/v1/me/tokens?expired_days=60").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 3);
    assert_response_tokens_contain_name(response_tokens, "bar");
    assert_response_tokens_contain_name(response_tokens, "ancient");
    assert_response_tokens_contain_name(response_tokens, "recent");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_tokens_exclude_revoked() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let id = user.as_model().id;

    let new_token = NewApiToken::builder().name("bar").user_id(id).build();
    let token1 = assert_ok!(new_token.insert(&mut conn).await);

    let new_token = NewApiToken::builder().name("baz").user_id(id).build();
    assert_ok!(new_token.insert(&mut conn).await);

    // List tokens expecting them all to be there.
    let response = user.get::<()>("/api/v1/me/tokens").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 2);

    // Revoke the first token.
    let response = user
        .delete::<()>(&format!("/api/v1/me/tokens/{}", token1.id))
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Check that we now have one less token being listed.
    let response = user.get::<()>("/api/v1/me/tokens").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    let response_tokens = json["api_tokens"].as_array().unwrap();
    assert_eq!(response_tokens.len(), 1);
    assert_eq!(response_tokens[0]["name"], json!("baz"));
}
