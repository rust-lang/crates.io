use conduit::StatusCode;
use crate::{TestApp, RequestHelper};

#[test]
fn metrics_endpoint_works() {
    let (_, anon) = TestApp::init().empty();

    let resp = anon.get::<()>("/api/private/metrics/service");
    assert_eq!(StatusCode::OK, resp.status());

    let resp = anon.get::<()>("/api/private/metrics/instance");
    assert_eq!(StatusCode::OK, resp.status());

    let resp = anon.get::<()>("/api/private/metrics/missing");
    assert_eq!(StatusCode::NOT_FOUND, resp.status());
}
