//! This module implements middleware to serve static files from the
//! specified directory.

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::{HeaderValue, Method, StatusCode, header};
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
    match serve_static(path, request).await {
        Ok(response) => response,
        Err(request) => next.run(request).await,
    }
}

/// Serve a static file from `path`, using the precompressed `.br`/`.gz`
/// variants when available and accepted.
///
/// Returns the original request back as [`Err`] when it should fall through to
/// the next handler: for non-GET/HEAD methods, for the `/` and `/index.html`
/// Jinja template (rendered by `frontend_html::serve`), and when no matching
/// file exists.
async fn serve_static<P: AsRef<Path>>(path: P, request: Request) -> Result<Response, Request> {
    if !matches!(*request.method(), Method::GET | Method::HEAD)
        || matches!(request.uri().path().as_bytes(), b"/" | b"/index.html")
    {
        return Err(request);
    }

    let mut static_req = Request::new(());
    *static_req.method_mut() = request.method().clone();
    *static_req.uri_mut() = request.uri().clone();
    *static_req.headers_mut() = request.headers().clone();

    let serve_dir = ServeDir::new(path).precompressed_br().precompressed_gzip();
    let Ok(response) = serve_dir.oneshot(static_req).await;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(request);
    }

    let mut response = response.map(Body::new);

    // FIXME: `ServeDir` does not set `Vary: Accept-Encoding` on precompressed
    // responses yet. Remove this once a tower-http release including
    // https://github.com/tower-rs/tower-http/pull/692 is available.
    response
        .headers_mut()
        .insert(header::VARY, HeaderValue::from_static("accept-encoding"));

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::serve_static;
    use axum::body::Body;
    use axum::extract::Request;
    use claims::{assert_err, assert_ok};
    use http::{StatusCode, header};

    #[tokio::test]
    async fn serves_file_with_vary_accept_encoding() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.js"), b"console.log(1)").unwrap();

        let request = Request::get("/app.js").body(Body::empty()).unwrap();
        let response = assert_ok!(serve_static(dir.path(), request).await);

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
        assert_err!(serve_static(dir.path(), request).await);
    }
}
