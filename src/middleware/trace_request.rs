use super::update_metrics::request_scheme;
use axum::extract::{MatchedPath, Request};
use axum::response::Response;
use http::header;
use std::time::Duration;
use tracing::field::Empty;
use tracing::{Span, info_span};

/// Creates a root span for an HTTP request, named after the matched route.
pub fn make_span(request: &Request) -> Span {
    let method = request.method();
    let uri = request.uri();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str);

    let name = match route {
        Some(route) => format!("{method} {route}"),
        None => method.to_string(),
    };

    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok());

    info_span!(
        parent: None,
        "http.request",
        otel.name = name,
        otel.kind = "server",
        otel.status_code = Empty,
        http.request.method = %method,
        http.route = route,
        http.response.status_code = Empty,
        error.type = Empty,
        url.path = uri.path(),
        url.query = uri.query(),
        url.scheme = request_scheme(request),
        user_agent.original = user_agent,
    )
}

/// Records the response status and marks server errors as failed spans.
pub fn on_response(response: &Response, _latency: Duration, span: &Span) {
    let status = response.status();
    // `tracing-opentelemetry` exports `u64` values as strings.
    span.record("http.response.status_code", i64::from(status.as_u16()));
    if status.is_server_error() {
        span.record("otel.status_code", "ERROR");
        span.record("error.type", status.as_str());
    }
}
