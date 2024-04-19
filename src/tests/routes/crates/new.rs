use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn daily_limit() {
    let (app, _, user) = TestApp::full().with_user();

    let max_daily_versions = app.as_inner().config.new_version_rate_limit.unwrap();
    for version in 1..=max_daily_versions {
        let crate_to_publish = PublishBuilder::new("foo_daily_limit", &format!("0.0.{version}"));
        user.async_publish_crate(crate_to_publish).await.good();
    }

    let crate_to_publish = PublishBuilder::new("foo_daily_limit", "1.0.0");
    let response = user.async_publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let json = response.json();
    assert_eq!(
        json["errors"][0]["detail"],
        "You have published too many versions of this crate in the last 24 hours"
    );
}
