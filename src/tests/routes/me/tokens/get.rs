use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn show_token_non_existing() {
    let url = "/api/v1/me/tokens/10086";
    let (_, _, user, _) = TestApp::init().with_token();
    user.get(url).await.assert_not_found();
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (_, _, user, token) = TestApp::init().with_token();
    let token = token.as_model();
    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.created_at" => "[datetime]",
    });
}
