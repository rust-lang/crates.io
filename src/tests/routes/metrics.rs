use crate::util::{MockAnonymousUser, MockRequestExt, Response};
use crate::{RequestHelper, TestApp};
use http::StatusCode;

#[test]
fn metrics_endpoint_works() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.metrics_authorization_token = Some("foobar".into()))
        .empty();

    let resp = request_metrics(&anon, "service", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = request_metrics(&anon, "instance", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = request_metrics(&anon, "missing", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn metrics_endpoint_wrong_auth() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.metrics_authorization_token = Some("secret".into()))
        .empty();

    // Wrong secret

    let resp = request_metrics(&anon, "service", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = request_metrics(&anon, "instance", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = request_metrics(&anon, "missing", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // No secret

    let resp = request_metrics(&anon, "service", None);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = request_metrics(&anon, "instance", None);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = request_metrics(&anon, "missing", None);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[test]
fn metrics_endpoint_auth_disabled() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.metrics_authorization_token = None)
        .empty();

    // Wrong secret

    let resp = request_metrics(&anon, "service", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = request_metrics(&anon, "instance", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = request_metrics(&anon, "missing", Some("foobar"));
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // No secret

    let resp = request_metrics(&anon, "service", None);
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = request_metrics(&anon, "instance", None);
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = request_metrics(&anon, "missing", None);
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

fn request_metrics(anon: &MockAnonymousUser, kind: &str, token: Option<&str>) -> Response<()> {
    let mut req = anon.get_request(&format!("/api/private/metrics/{kind}"));
    if let Some(token) = token {
        req.header("Authorization", &format!("Bearer {token}"));
    }
    anon.run(req)
}
