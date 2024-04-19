use crate::util::{RequestHelper, TestApp};
use http::{Method, StatusCode};

#[tokio::test(flavor = "multi_thread")]
async fn head_method_works() {
    let (_, anon) = TestApp::init().empty();

    let req = anon.request_builder(Method::HEAD, "/api/v1/summary");
    let res = anon.run::<()>(req).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text(), "");
}

#[tokio::test(flavor = "multi_thread")]
async fn head_method_works_for_404() {
    let (_, anon) = TestApp::init().empty();

    let req = anon.request_builder(Method::HEAD, "/unknown");
    let res = anon.run::<()>(req).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(res.text(), "");
}
