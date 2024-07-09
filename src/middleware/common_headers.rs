use crate::app::AppState;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::headers::{CacheControl, Expires, HeaderMapExt};
use http::{header, HeaderMap, HeaderValue};
use std::time::{Duration, SystemTime};

// see http://nginx.org/en/docs/http/ngx_http_headers_module.html#add_header
const NGINX_SUCCESS_CODES: [u16; 10] = [200, 201, 204, 206, 301, 203, 303, 304, 307, 308];

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);
const ONE_YEAR: Duration = Duration::from_secs(365 * 24 * 60 * 60);

pub async fn add_common_headers(
    state: AppState,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let v = HeaderValue::from_static;

    let mut headers = HeaderMap::new();

    let path = request.uri().path();

    const STATIC_FILES: [&str; 5] = [
        "/github-redirect.html",
        "/favicon.ico",
        "/robots.txt",
        "/opensearch.xml",
        "/.well-known/security.txt",
    ];
    if STATIC_FILES.contains(&path) {
        expires(&mut headers, ONE_DAY);
    }

    if path.starts_with("/assets/") {
        expires(&mut headers, 10 * ONE_YEAR);
    }

    let response = next.run(request).await;

    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v("*"));
    headers.insert(header::STRICT_TRANSPORT_SECURITY, v("max-age=31536000"));

    if NGINX_SUCCESS_CODES.contains(&response.status().as_u16()) {
        headers.insert(header::X_CONTENT_TYPE_OPTIONS, v("nosniff"));
        headers.insert(header::X_FRAME_OPTIONS, v("SAMEORIGIN"));
        headers.insert(header::X_XSS_PROTECTION, v("0"));
        if let Some(ref csp) = state.config.content_security_policy {
            headers.insert(header::CONTENT_SECURITY_POLICY, csp.clone());
        }
        headers.insert(header::VARY, v("Accept, Accept-Encoding, Cookie"));
    }

    (headers, response)
}

fn expires(headers: &mut HeaderMap, cache_duration: Duration) {
    headers.typed_insert(Expires::from(SystemTime::now() + cache_duration));
    headers.typed_insert(
        CacheControl::new()
            .with_public()
            .with_max_age(cache_duration),
    );
}
