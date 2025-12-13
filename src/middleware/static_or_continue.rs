//! This module implements middleware to serve static files from the
//! specified directory.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::uri::PathAndQuery;
use http::{Method, StatusCode, Uri};
use std::path::Path;
use tower::ServiceExt;
use tower_http::services::ServeDir;

pub async fn serve_local_uploads(request: Request, next: Next) -> Response {
    serve("local_uploads", None, request, next).await
}

pub async fn serve_dist(request: Request, next: Next) -> Response {
    serve("dist", None, request, next).await
}

pub async fn serve_svelte(request: Request, next: Next) -> Response {
    serve("svelte/build", Some("/svelte"), request, next).await
}

async fn serve<P: AsRef<Path>>(
    path: P,
    strip_prefix: Option<&str>,
    request: Request,
    next: Next,
) -> Response {
    // index.html is a Jinja template, which is to be rendered by `ember_html::serve_html`.
    if matches!(*request.method(), Method::GET | Method::HEAD)
        && !matches!(request.uri().path().as_bytes(), b"/" | b"/index.html")
        && strip_prefix.is_none_or(|prefix| request.uri().path().starts_with(prefix))
    {
        let mut static_req = Request::new(());
        *static_req.method_mut() = request.method().clone();
        *static_req.uri_mut() = request.uri().clone();
        *static_req.headers_mut() = request.headers().clone();

        if let Some(prefix) = strip_prefix
            && let Some(new_path) = request.uri().path().strip_prefix(prefix)
            && let Some(new_uri) = replace_uri_path(request.uri(), new_path)
        {
            *static_req.uri_mut() = new_uri;
        }

        let serve_dir = ServeDir::new(path).precompressed_br().precompressed_gzip();
        let Ok(response) = serve_dir.oneshot(static_req).await;
        if response.status() != StatusCode::NOT_FOUND {
            return response.map(axum::body::Body::new);
        }
    }

    next.run(request).await
}

/// Replaces the path component of a URI while preserving the query string.
fn replace_uri_path(uri: &Uri, new_path: &str) -> Option<Uri> {
    let new_path = if new_path.is_empty() { "/" } else { new_path };

    let new_path_and_query = match uri.query() {
        Some(query) => format!("{new_path}?{query}"),
        None => new_path.to_owned(),
    };

    let path_and_query = PathAndQuery::try_from(new_path_and_query).ok()?;
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(path_and_query);
    Uri::from_parts(parts).ok()
}
