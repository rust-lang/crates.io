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
