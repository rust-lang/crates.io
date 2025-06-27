//! Debug middleware that prints debug info to stdout

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use tracing::debug;

pub async fn debug_requests(req: Request, next: Next) -> impl IntoResponse {
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
