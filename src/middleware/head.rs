//! Middleware that proxies HEAD requests into a GET request then throws away the body

use axum::body::BoxBody;
use axum::middleware::Next;
use axum::response::Response;
use http::{Method, Request};

pub async fn support_head_requests<B>(mut req: Request<B>, next: Next<B>) -> Response {
    if req.method() != Method::HEAD {
        return next.run(req).await;
    }

    *req.method_mut() = Method::GET;
    let mut response = next.run(req).await;
    *response.body_mut() = BoxBody::default();
    response
}
