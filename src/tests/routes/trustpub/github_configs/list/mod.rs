use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

mod by_crate;
mod by_user;

pub const URL: &str = "/api/v1/trusted_publishing/github_configs";

#[tokio::test(flavor = "multi_thread")]
async fn test_no_query_param() -> anyhow::Result<()> {
    let (_, _, cookie_client) = TestApp::full().with_user().await;

    let response = cookie_client.get::<()>(URL).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Must specify either `crate` or `user_id` query parameter"}]}"#);

    Ok(())
}
