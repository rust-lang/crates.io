use crate::tests::util::{RequestHelper, Response, TestApp};
use http::StatusCode;
use insta::assert_snapshot;

mod publish_notifications;

pub trait MockEmailHelper: RequestHelper {
    // TODO: I don't like the name of this method or `update_email` on the `MockCookieUser` impl;
    // this is starting to look like a builder might help?
    // I want to explore alternative abstractions in any case.
    async fn update_email_more_control(&self, user_id: i32, email: Option<&str>) -> Response<()> {
        let body = json!({"user": { "email": email }});
        let url = format!("/api/v1/users/{user_id}");
        self.put(&url, body.to_string()).await
    }
}

impl MockEmailHelper for crate::tests::util::MockCookieUser {}
impl MockEmailHelper for crate::tests::util::MockAnonymousUser {}

impl crate::tests::util::MockCookieUser {
    pub async fn update_email(&self, email: &str) {
        let model = self.as_model();
        let response = self.update_email_more_control(model.id, Some(email)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
    }
}

/// Given a crates.io user, check to make sure that the user
/// cannot add to the database an empty string or null as
/// their email. If an attempt is made, update_user.rs will
/// return an error indicating that an empty email cannot be
/// added.
///
/// This is checked on the frontend already, but I'd like to
/// make sure that a user cannot get around that and delete
/// their email by adding an empty string.
#[tokio::test(flavor = "multi_thread")]
async fn test_empty_email_not_added() {
    let (_app, _anon, user) = TestApp::init().with_user();
    let model = user.as_model();

    let response = user.update_email_more_control(model.id, Some("")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"empty email rejected"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_empty() {
    let (_app, _anon, user) = TestApp::init().with_user();
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let payload = json!({"user": {}});
    let response = user.put::<()>(&url, payload.to_string()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_nulls() {
    let (_app, _anon, user) = TestApp::init().with_user();
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let payload = json!({"user": { "email": null }});
    let response = user.put::<()>(&url, payload.to_string()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
}

/// Check to make sure that neither other signed in users nor anonymous users can edit another
/// user's email address.
///
/// If an attempt is made, update_user.rs will return an error indicating that the current user
/// does not match the requested user.
#[tokio::test(flavor = "multi_thread")]
async fn test_other_users_cannot_change_my_email() {
    let (app, anon, user) = TestApp::init().with_user();
    let another_user = app.db_new_user("not_me");
    let another_user_model = another_user.as_model();

    let response = user
        .update_email_more_control(
            another_user_model.id,
            Some("pineapple@pineapples.pineapple"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    let response = anon
        .update_email_more_control(
            another_user_model.id,
            Some("pineapple@pineapples.pineapple"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_email_address() {
    let (_app, _, user) = TestApp::init().with_user();
    let model = user.as_model();

    let response = user.update_email_more_control(model.id, Some("foo")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid email address"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_json() {
    let (_app, _anon, user) = TestApp::init().with_user();
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let response = user.put::<()>(&url, r#"{ "user": foo }"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: user: expected ident at line 1 column 12"}]}"#);
}
