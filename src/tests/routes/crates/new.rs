use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[test]
fn daily_limit() {
    let (app, _, user) = TestApp::full().with_user();

    let max_daily_versions = app.as_inner().config.new_version_rate_limit.unwrap();
    for version in 1..=max_daily_versions {
        let crate_to_publish = PublishBuilder::new("foo_daily_limit", &format!("0.0.{version}"));
        user.publish_crate(crate_to_publish).good();
    }

    let crate_to_publish = PublishBuilder::new("foo_daily_limit", "1.0.0");
    let response = user.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let json = response.json();
    assert_eq!(
        json["errors"][0]["detail"],
        "You have published too many versions of this crate in the last 24 hours"
    );
}
