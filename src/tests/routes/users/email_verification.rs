use super::emails::MockEmailHelper;
use crate::tests::util::{RequestHelper, Response, TestApp};
use insta::assert_snapshot;

pub trait MockEmailVerificationHelper: RequestHelper {
    async fn resend_confirmation(&self, user_id: i32, email_id: i32) -> Response<()> {
        let url = format!("/api/v1/users/{user_id}/emails/{email_id}/resend");
        self.put(&url, &[] as &[u8]).await
    }
}

impl MockEmailVerificationHelper for crate::tests::util::MockCookieUser {}
impl MockEmailVerificationHelper for crate::tests::util::MockAnonymousUser {}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_auth() {
    let (app, anon, user) = TestApp::init().with_user().await;

    let response = anon.resend_confirmation(user.as_model().id, 1).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_wrong_user() {
    let (app, _anon, user) = TestApp::init().with_user().await;
    let user2 = app.db_new_user("bar").await;
    let response = user.resend_confirmation(user2.as_model().id, 1).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);
    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() {
    let (app, _anon, user) = TestApp::init().with_user().await;

    // Add an email to the user
    let response = user.add_email(user.as_model().id, "user@example.com").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"id":2,"email":"user@example.com","verified":false,"verification_email_sent":true,"primary":false}"#);

    let response = user
        .resend_confirmation(
            user.as_model().id,
            response.json()["id"].as_u64().unwrap() as i32,
        )
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
    assert_snapshot!(app.emails_snapshot().await);
}
