use crate::util::{RequestHelper, TestApp};
#[tokio::test(flavor = "multi_thread")]
async fn show_token_non_existing() {
    let url = "/api/v1/me/tokens/10086";
    let (_, _, user, _) = TestApp::init().with_token();
    user.get(url).await.assert_not_found();
}
