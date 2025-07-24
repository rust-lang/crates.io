use crate::tests::util::{RequestHelper, Response, TestApp};
use insta::assert_snapshot;
use serde_json::json;

pub trait MockEmailHelper: RequestHelper {
    async fn add_email(&self, user_id: i32, email: &str) -> Response<()> {
        let body = json!({"email": email});
        let url = format!("/api/v1/users/{user_id}/emails");
        self.post(&url, body.to_string()).await
    }

    async fn delete_email(&self, user_id: i32, email_id: i32) -> Response<()> {
        let url = format!("/api/v1/users/{user_id}/emails/{email_id}");
        self.delete(&url).await
    }

    async fn enable_notifications(&self, user_id: i32, email_id: i32) -> Response<()> {
        let url = format!("/api/v1/users/{user_id}/emails/{email_id}/notifications");
        self.put(&url, "").await
    }
}

impl MockEmailHelper for crate::tests::util::MockCookieUser {}
impl MockEmailHelper for crate::tests::util::MockAnonymousUser {}

/// Given a crates.io user, check that the user can add an email address
/// to their profile, and that the email address is then returned by the
/// `/me` endpoint.
#[tokio::test(flavor = "multi_thread")]
async fn test_email_add() -> anyhow::Result<()> {
    let (_app, _anon, user) = TestApp::init().with_user().await;

    let json = user.show_me().await;
    assert_eq!(json.user.emails.len(), 1);
    assert_eq!(json.user.emails.first().unwrap().email, "foo@example.com");

    let response = user.add_email(json.user.id, "bar@example.com").await;
    let json = user.show_me().await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"id":2,"email":"bar@example.com","verified":false,"verification_email_sent":true,"send_notifications":false}"#);
    assert_eq!(json.user.emails.len(), 2);
    assert!(
        json.user
            .emails
            .iter()
            .any(|e| e.email == "bar@example.com")
    );
    assert!(
        json.user
            .emails
            .iter()
            .find(|e| e.email == "foo@example.com")
            .unwrap()
            .send_notifications
    );

    Ok(())
}

/// Given a crates.io user, check to make sure that the user
/// cannot add to the database an empty string or null as
/// their email. If an attempt is made, the emails controller
/// will return an error indicating that an empty email cannot be
/// added.
///
/// This is checked on the frontend already, but I'd like to
/// make sure that a user cannot get around that and delete
/// their email by adding an empty string.
#[tokio::test(flavor = "multi_thread")]
async fn test_empty_email_not_added() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.add_email(model.id, "").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"empty email rejected"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_empty_json() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}/emails", model.id);
    let payload = json!({});
    let response = user.post::<()>(&url, payload.to_string()).await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_null_email() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}/emails", model.id);
    let payload = json!({ "email": null });
    let response = user.post::<()>(&url, payload.to_string()).await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
}

/// Check to make sure that neither other signed in users nor anonymous users can add an
/// email address to another user's account.
///
/// If an attempt is made, the emails controller will return an error indicating that the
/// current user does not match the requested user.
#[tokio::test(flavor = "multi_thread")]
async fn test_other_users_cannot_change_my_email() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let another_user = app.db_new_user("not_me").await;
    let another_user_model = another_user.as_model();

    let response = user
        .add_email(another_user_model.id, "pineapple@pineapples.pineapple")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    let response = anon
        .add_email(another_user_model.id, "pineapple@pineapples.pineapple")
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_email_address() {
    let (_app, _, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.add_email(model.id, "foo").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid email address"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_json() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}/emails", model.id);
    let response = user.post::<()>(&url, r#"{ "user": foo }"#).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: user: expected ident at line 1 column 12"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delete_email_invalid_id() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.delete_email(model.id, 0).await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_other_users_cannot_delete_my_email() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let another_user = app.db_new_user("not_me").await;
    let another_user_model = another_user.as_model();

    let response = user.delete_email(another_user_model.id, 0).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    let response = anon.delete_email(another_user_model.id, 0).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cannot_delete_my_notification_email() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    // Attempt to delete the email address that is used for notifications
    let response = user.delete_email(model.id, 1).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"cannot delete email that receives notifications"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_can_delete_an_alternative_email() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    // Add an alternative email address
    let response = user.add_email(model.id, "potato3@example.com").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"id":2,"email":"potato3@example.com","verified":false,"verification_email_sent":true,"send_notifications":false}"#);

    // Attempt to delete the alternative email address
    let response = user.delete_email(model.id, 2).await;
    assert_snapshot!(response.status(), @"200 OK");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_enable_notifications_invalid_id() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.enable_notifications(model.id, 0).await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_other_users_cannot_enable_my_notifications() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let another_user = app.db_new_user("not_me").await;
    let another_user_model = another_user.as_model();

    let response = user.enable_notifications(another_user_model.id, 1).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    let response = anon.enable_notifications(another_user_model.id, 1).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}
