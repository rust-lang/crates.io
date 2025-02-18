use crate::models::token::{CrateScope, EndpointScope, NewApiToken};
use crate::tests::util::{RequestHelper, TestApp};
use chrono::{Duration, Utc};
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn show_token_non_existing() {
    let url = "/api/v1/me/tokens/10086";
    let (_, _, user, _) = TestApp::init().with_token().await;
    user.get(url).await.assert_not_found();
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (_, _, user, token) = TestApp::init().with_token().await;
    let token = token.as_model();
    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.created_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_token_with_scopes() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user_model = user.as_model();
    let id = user_model.id;

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
        .expired_at(Utc::now() - Duration::days(31))
        .build();
    let token = assert_ok!(new_token.insert(&mut conn).await);

    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.created_at" => "[datetime]",
        ".api_token.expired_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_with_anonymous_user() {
    let url = "/api/v1/me/tokens/1";
    let (_, anon) = TestApp::init().empty().await;
    anon.get(url).await.assert_forbidden();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_other_user_token() {
    let (app, _, user1) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user2 = app.db_new_user("baz").await;
    let user2 = user2.as_model();

    let new_token = NewApiToken::builder().name("bar").user_id(user2.id).build();
    let token = assert_ok!(new_token.insert(&mut conn).await);

    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user1.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
