//! This module implements middleware to serve static files from the
//! specified directory.

use axum::middleware::Next;
use axum::response::Response;
use http::{Method, Request, StatusCode};
use std::path::Path;
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn serve_local_uploads<B>(request: Request<B>, next: Next<B>) -> Response {
    serve("local_uploads", request, next).await
}

pub async fn serve_dist<B>(request: Request<B>, next: Next<B>) -> Response {
    serve("dist", request, next).await
}

async fn serve<P: AsRef<Path>, B>(path: P, request: Request<B>, next: Next<B>) -> Response {
    if request.method() == Method::GET || request.method() == Method::HEAD {
        let mut static_req = Request::new(());
        *static_req.method_mut() = request.method().clone();
        *static_req.uri_mut() = request.uri().clone();
        *static_req.headers_mut() = request.headers().clone();

        if let Ok(response) = ServeDir::new(path).oneshot(static_req).await {
            if response.status() != StatusCode::NOT_FOUND {
                return response.map(axum::body::boxed);
            }
        }
    }

    next.run(request).await
}
