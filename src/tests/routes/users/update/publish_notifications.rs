use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_unsubscribe_and_resubscribe() {
    let (app, _anon, cookie, token) = TestApp::full().with_token();

    let user_url = format!("/api/v1/users/{}", cookie.as_model().id);

    // Publish a crate to trigger an initial publish email
    let pb = PublishBuilder::new("foo", "1.0.0");
    let response = token.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Assert that the user gets an initial publish email
    assert_snapshot!(app.emails_snapshot());

    // Unsubscribe from publish notifications
    let payload = json!({"user": { "publish_notifications": false }});
    let response = cookie.put::<()>(&user_url, payload.to_string()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);

    // Assert that the user gets an unsubscribe email
    assert_snapshot!(app.emails_snapshot());

    // Publish the same crate again to check that the user doesn't get a publish email
    let pb = PublishBuilder::new("foo", "1.1.0");
    let response = token.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Assert that the user did not get a publish email this time
    assert_snapshot!(app.emails_snapshot());

    // Resubscribe to publish notifications
    let payload = json!({"user": { "publish_notifications": true }});
    let response = cookie.put::<()>(&user_url, payload.to_string()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);

    // Publish the same crate again to check that the user doesn't get a publish email
    let pb = PublishBuilder::new("foo", "1.2.0");
    let response = token.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Assert that the user got a publish email again
    assert_snapshot!(app.emails_snapshot());
}
