use crate::tests::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_no_auth() {
    let (app, anon, user) = TestApp::init().with_user().await;

    let url = format!("/api/v1/users/{}/resend", user.as_model().id);
    let response = anon.put::<()>(&url, "").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_wrong_user() {
    let (app, _anon, user) = TestApp::init().with_user().await;
    let user2 = app.db_new_user("bar").await;

    let url = format!("/api/v1/users/{}/resend", user2.as_model().id);
    let response = user.put::<()>(&url, "").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() {
    let (app, _anon, user) = TestApp::init().with_user().await;

    let url = format!("/api/v1/users/{}/resend", user.as_model().id);
    let response = user.put::<()>(&url, "").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);

    assert_snapshot!(app.emails_snapshot().await);
}
