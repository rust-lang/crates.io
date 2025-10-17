use crate::TestApp;
use crate::util::{MockRequestExt, RequestHelper};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_with_matching_origin() {
    let (_, _, cookie) = TestApp::init()
        .with_config(|server| {
            server.allowed_origins = "https://crates.io".parse().unwrap();
        })
        .with_user()
        .await;

    let mut request = cookie.get_request("/api/v1/me");
    request.header("Origin", "https://crates.io");

    let response = cookie.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_with_unknown_origin() {
    let (_, _, cookie) = TestApp::init()
        .with_config(|server| {
            server.allowed_origins = "https://crates.io".parse().unwrap();
        })
        .with_user()
        .await;

    let mut request = cookie.get_request("/api/v1/me");
    request.header("Origin", "https://evil.hacker.io");

    let response = cookie.run::<()>(request).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid origin header"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_with_multiple_origins() {
    let (_, _, cookie) = TestApp::init()
        .with_config(|server| {
            server.allowed_origins = "https://crates.io".parse().unwrap();
        })
        .with_user()
        .await;

    let mut request = cookie.get_request("/api/v1/me");
    request.header("Origin", "https://evil.hacker.io");
    request.header("Origin", "https://crates.io");

    let response = cookie.run::<()>(request).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid origin header"}]}"#);
}
