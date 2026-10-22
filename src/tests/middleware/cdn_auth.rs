use crate::util::{MockAnonymousUser, MockRequestExt, RequestHelper, Response, TestApp};
use crates_io::middleware::cdn_auth::X_CRATES_ORIGIN_AUTH;
use http::Method;
use insta::assert_snapshot;

const SUMMARY_URL: &str = "/api/v1/summary";
const CDN_AUTH_TOKEN: &str = "secret";

async fn app_with_token() -> MockAnonymousUser {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.cdn_auth_token = Some(CDN_AUTH_TOKEN.into()))
        .empty()
        .await;
    anon
}

async fn get_with_header(anon: &MockAnonymousUser, value: &str) -> Response<()> {
    let mut request = anon.request_builder(Method::GET, SUMMARY_URL);
    request.header(X_CRATES_ORIGIN_AUTH, value);
    anon.run(request).await
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_header_is_rejected() {
    let anon = app_with_token().await;

    let response = anon.get::<()>(SUMMARY_URL).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Direct access to the origin server is not allowed"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_header_is_rejected() {
    let anon = app_with_token().await;

    let response = get_with_header(&anon, "wrong").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Direct access to the origin server is not allowed"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn valid_header_is_accepted() {
    let anon = app_with_token().await;

    let response = get_with_header(&anon, CDN_AUTH_TOKEN).await;
    assert_snapshot!(response.status(), @"200 OK");
}

#[tokio::test(flavor = "multi_thread")]
async fn no_check_without_configured_token() {
    let (_, anon) = TestApp::init().empty().await;

    let response = anon.get::<()>(SUMMARY_URL).await;
    assert_snapshot!(response.status(), @"200 OK");
}
