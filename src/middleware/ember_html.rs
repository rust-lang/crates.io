//! Serve the Ember.js frontend HTML
//!
//! Paths intended for the inner `api_handler` are passed along to the remaining middleware layers
//! as normal. Requests not intended for the backend will be served HTML to boot the Ember.js
//! frontend. During local development, if so configured, these requests will instead be proxied to
//! Ember FastBoot (`node ./fastboot.js`).
//!
//! For now, there is an additional check to see if the `Accept` header contains "html". This is
//! likely to be removed in the future.

use crate::app::AppState;
use anyhow::ensure;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{header, Request, StatusCode};
use reqwest::Client;
use std::fmt::Write;
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub async fn serve_html<B: Send + 'static>(
    state: AppState,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let path = &request.uri().path();

    // The "/git/" prefix is only used in development (when within a docker container).
    //
    // The other prefixes must be kept in sync with the `proxyPaths` defined in `server/index.js`
    // and the nginx configuration.
    if path.starts_with("/admin/") || path.starts_with("/api/") || path.starts_with("/git/") {
        next.run(request).await
    } else {
        if let Some(client) = &state.fastboot_client {
            // During local fastboot development, forward requests to the local fastboot server.
            // In prodution, including when running with fastboot, nginx proxies the requests
            // to the correct endpoint and requests should never make it here.
            return proxy_to_fastboot(client, request)
                .await
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }

        if request
            .headers()
            .get_all(header::ACCEPT)
            .iter()
            .any(|val| val.to_str().unwrap_or_default().contains("html"))
        {
            // Serve static Ember page to bootstrap the frontend
            ServeFile::new("dist/index.html")
                .oneshot(request)
                .await
                .map(|response| response.map(axum::body::boxed))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        } else {
            // Return a 404 to crawlers that don't send `Accept: text/hml`.
            // This is to preserve legacy behavior and will likely change.
            // Most of these crawlers probably won't execute our frontend JS anyway, but
            // it would be nice to bootstrap the app for crawlers that do execute JS.
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Proxy to the fastboot server in development mode
///
/// This handler is somewhat hacky, and is not intended for usage in production.
///
/// # Panics
///
/// This function can panic and should only be used in development mode.
async fn proxy_to_fastboot<B>(client: &Client, req: Request<B>) -> anyhow::Result<Response> {
    ensure!(
        req.method() == http::Method::GET,
        "Only support GET but request method was {}",
        req.method()
    );

    let mut url = format!("http://127.0.0.1:9000{}", req.uri().path());
    if let Some(query) = req.uri().query() {
        write!(url, "?{query}")?;
    }

    let fastboot_response = client
        .request(req.method().into(), &*url)
        .headers(req.headers().clone())
        .send()
        .await?;

    let status = fastboot_response.status();
    let headers = fastboot_response.headers().clone();
    let bytes = fastboot_response.bytes().await?;

    Ok((status, headers, bytes).into_response())
}
