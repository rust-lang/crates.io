use crate::middleware::log_request::RequestLogExt;
use crate::util::errors::forbidden;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::HeaderName;
use secrecy::{ExposeSecret, SecretString};

pub const X_CRATES_ORIGIN_AUTH: HeaderName = HeaderName::from_static("x-crates-origin-auth");

/// Rejects requests that did not pass through one of our CDNs.
///
/// Fastly and CloudFront send the `X-Crates-Origin-Auth` header with the
/// secret from the `CRATES_ORIGIN_AUTH_TOKEN` environment variable. Requests
/// sent directly to the Heroku app do not have it.
pub async fn middleware(State(token): State<SecretString>, req: Request, next: Next) -> Response {
    let cause = match req.headers().get(X_CRATES_ORIGIN_AUTH) {
        None => "missing CDN auth header",
        Some(value) if value == token.expose_secret() => return next.run(req).await,
        Some(_) => "invalid CDN auth header",
    };

    req.request_log().add("cause", cause);
    forbidden("Direct access to the origin server is not allowed").into_response()
}
