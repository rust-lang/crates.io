//! Serve the Ember.js frontend HTML
//!
//! Paths intended for the inner `api_handler` are passed along to the remaining middleware layers
//! as normal. Requests not intended for the backend will be served HTML to boot the Ember.js
//! frontend.
//!
//! For now, there is an additional check to see if the `Accept` header contains "html". This is
//! likely to be removed in the future.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{header, StatusCode};
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub async fn serve_html(request: Request, next: Next) -> Response {
    let path = &request.uri().path();

    // The "/git/" prefix is only used in development (when within a docker container)
    if path.starts_with("/api/") || path.starts_with("/git/") {
        next.run(request).await
    } else if request
        .headers()
        .get_all(header::ACCEPT)
        .iter()
        .any(|val| val.to_str().unwrap_or_default().contains("html"))
    {
        // Serve static Ember page to bootstrap the frontend
        ServeFile::new("dist/index.html")
            .oneshot(request)
            .await
            .map(|response| response.map(axum::body::Body::new))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        // Return a 404 to crawlers that don't send `Accept: text/hml`.
        // This is to preserve legacy behavior and will likely change.
        // Most of these crawlers probably won't execute our frontend JS anyway, but
        // it would be nice to bootstrap the app for crawlers that do execute JS.
        StatusCode::NOT_FOUND.into_response()
    }
}
