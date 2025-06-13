use crate::tests::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon.get::<()>("/api/private/session/authorize").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: missing field `code`"}]}"#);
}
