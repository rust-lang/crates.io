use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon.get::<()>("/api/private/session/authorize").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: missing field `code`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn post_rejects_invalid_state() {
    let (_, anon) = TestApp::init().empty().await;
    let body = r#"{"code":"901dd10e07c7e9fa1cd5","state":"fYcUY3FMdUUz00FC7vLT7A"}"#;
    let response = anon
        .post::<()>("/api/private/session/authorize", body)
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid state parameter"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn post_needs_data() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon
        .post::<()>("/api/private/session/authorize", "{}")
        .await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `code` at line 1 column 2"}]}"#);
}
