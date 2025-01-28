//! This module implements middleware to serve static files from the
//! specified directory.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::{Method, StatusCode};
use std::path::Path;
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn serve_local_uploads(request: Request, next: Next) -> Response {
    serve("local_uploads", request, next).await
}

pub async fn serve_dist(request: Request, next: Next) -> Response {
    serve("dist", request, next).await
}

async fn serve<P: AsRef<Path>>(path: P, request: Request, next: Next) -> Response {
    // index.html is a Jinja template, which is to be rendered by `ember_html::serve_html`.
    if matches!(*request.method(), Method::GET | Method::HEAD)
        && !matches!(request.uri().path().as_bytes(), b"/" | b"/index.html")
    {
        let mut static_req = Request::new(());
        *static_req.method_mut() = request.method().clone();
        *static_req.uri_mut() = request.uri().clone();
        *static_req.headers_mut() = request.headers().clone();

        let serve_dir = ServeDir::new(path).precompressed_br().precompressed_gzip();
        let Ok(response) = serve_dir.oneshot(static_req).await;
        if response.status() != StatusCode::NOT_FOUND {
            return response.map(axum::body::Body::new);
        }
    }

    next.run(request).await
}
