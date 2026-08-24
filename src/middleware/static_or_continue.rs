//! This module implements middleware to serve static files from the
//! specified directory.

use axum::body::Body;
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

pub async fn serve_svelte(request: Request, next: Next) -> Response {
    serve("svelte/build", request, next).await
}

async fn serve<P: AsRef<Path>>(path: P, request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    match serve_static(path, &parts).await {
        Some(response) => response,
        None => next.run(Request::from_parts(parts, body)).await,
    }
}

/// Serves a static file from `path`, using the precompressed `.br`/`.gz`
/// variants when available and accepted.
///
/// Returns [`None`] when the request should fall through to the next handler:
/// for non-GET/HEAD methods, for the `/` and `/index.html` Jinja template
/// (rendered by `frontend_html::serve`), and when no matching file exists.
async fn serve_static<P: AsRef<Path>>(path: P, parts: &http::request::Parts) -> Option<Response> {
    if !matches!(parts.method, Method::GET | Method::HEAD)
        || matches!(parts.uri.path().as_bytes(), b"/" | b"/index.html")
    {
        return None;
    }

    let mut static_req = Request::new(());
    *static_req.method_mut() = parts.method.clone();
    *static_req.uri_mut() = parts.uri.clone();
    *static_req.headers_mut() = parts.headers.clone();

    let serve_dir = ServeDir::new(path).precompressed_br().precompressed_gzip();
    let Ok(response) = serve_dir.oneshot(static_req).await;
    if response.status() == StatusCode::NOT_FOUND {
        return None;
    }

    Some(response.map(Body::new))
}

#[cfg(test)]
mod tests {
    use super::serve_static;
    use axum::body::Body;
    use axum::extract::Request;
    use claims::{assert_none, assert_some};
    use http::{StatusCode, header};

    #[tokio::test]
    async fn serves_file_with_vary_accept_encoding() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.js"), b"console.log(1)").unwrap();

        let request = Request::get("/app.js").body(Body::empty()).unwrap();
        let (parts, _) = request.into_parts();
        let response = assert_some!(serve_static(dir.path(), &parts).await);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::VARY).unwrap(),
            "accept-encoding"
        );
    }

    #[tokio::test]
    async fn falls_through_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();

        let request = Request::get("/missing.js").body(Body::empty()).unwrap();
        let (parts, _) = request.into_parts();
        assert_none!(serve_static(dir.path(), &parts).await);
    }
}
