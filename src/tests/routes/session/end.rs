use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn returns_ok_response() {
    let (_, _, user) = TestApp::init().with_user().await;

    let response = user.delete::<()>("/api/private/session").await;

    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
}
