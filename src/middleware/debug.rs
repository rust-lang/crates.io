//! Debug middleware that prints debug info to stdout

use axum::middleware::Next;
use axum::response::IntoResponse;
use http::Request;

pub async fn debug_requests<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    debug!("  version: {:?}", req.version());
    debug!("  method: {:?}", req.method());
    debug!("  path: {}", req.uri().path());
    debug!("  query_string: {:?}", req.uri().query());
    for (k, ref v) in req.headers().iter() {
        debug!("  hdr: {}={:?}", k, v);
    }

    let response = next.run(req).await;

    debug!("  <- {:?}", response.status());
    for (k, v) in response.headers().iter() {
        debug!("  <- {k} {v:?}");
    }

    response
}
