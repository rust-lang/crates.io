use crate::app::AppState;
use axum::middleware::Next;
use axum::response::IntoResponse;
use http::{header, HeaderMap, HeaderValue, Request};

// see http://nginx.org/en/docs/http/ngx_http_headers_module.html#add_header
const NGINX_SUCCESS_CODES: [u16; 10] = [200, 201, 204, 206, 301, 203, 303, 304, 307, 308];

#[instrument(skip_all)]
pub async fn add_common_headers<B: Send + 'static>(
    state: AppState,
    request: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let response = next.run(request).await;

    let v = HeaderValue::from_static;

    let mut headers = HeaderMap::new();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v("*"));
    headers.insert(header::STRICT_TRANSPORT_SECURITY, v("max-age=31536000"));

    if NGINX_SUCCESS_CODES.contains(&response.status().as_u16()) {
        let cdn_prefix = state.config.storage.cdn_prefix.as_ref();
        let cdn_domain = cdn_prefix.map(|cdn_prefix| format!("https://{cdn_prefix}"));
        let cdn_domain = cdn_domain.unwrap_or_default();

        let csp = format!(
            "default-src 'self'; \
            connect-src 'self' *.ingest.sentry.io https://docs.rs https://play.rust-lang.org {cdn_domain}; \
            script-src 'self' 'unsafe-eval' 'sha256-n1+BB7Ckjcal1Pr7QNBh/dKRTtBQsIytFodRiIosXdE='; \
            style-src 'self' 'unsafe-inline' https://code.cdn.mozilla.net; \
            font-src https://code.cdn.mozilla.net; \
            img-src *; \
            object-src 'none'"
        );

        headers.insert(header::X_CONTENT_TYPE_OPTIONS, v("nosniff"));
        headers.insert(header::X_FRAME_OPTIONS, v("SAMEORIGIN"));
        headers.insert(header::X_XSS_PROTECTION, v("0"));
        headers.insert(header::CONTENT_SECURITY_POLICY, csp.parse().unwrap());
        headers.insert(header::VARY, v("Accept, Accept-Encoding, Cookie"));
    }

    (headers, response)
}
