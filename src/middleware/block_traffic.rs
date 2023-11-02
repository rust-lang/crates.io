//! Middleware that blocks requests if a header matches the given list
//!
//! To use, set the `BLOCKED_TRAFFIC` environment variable to a comma-separated list of pairs
//! containing a header name, an equals sign, and the name of another environment variable that
//! contains the values of that header that should be blocked. For example, set `BLOCKED_TRAFFIC`
//! to `User-Agent=BLOCKED_UAS,X-Real-Ip=BLOCKED_IPS`, `BLOCKED_UAS` to `curl/7.54.0,cargo 1.36.0
//! (c4fcfb725 2019-05-15)`, and `BLOCKED_IPS` to `192.168.0.1,127.0.0.1` to block requests from
//! the versions of curl or Cargo specified or from either of the IPs (values are nonsensical
//! examples). Values of the headers must match exactly.

use crate::app::AppState;
use crate::middleware::log_request::RequestLogExt;
use crate::util::errors::RouteBlocked;
use axum::extract::MatchedPath;
use axum::middleware::Next;
use axum::response::IntoResponse;
use http::StatusCode;

pub async fn block_by_header<B>(
    state: AppState,
    req: http::Request<B>,
    next: Next<B>,
) -> axum::response::Response {
    let blocked_traffic = &state.config.blocked_traffic;

    for (header_name, blocked_values) in blocked_traffic {
        let has_blocked_value = req
            .headers()
            .get_all(header_name)
            .iter()
            .any(|value| blocked_values.iter().any(|v| v == value));
        if has_blocked_value {
            let cause = format!("blocked due to contents of header {header_name}");
            req.request_log().add("cause", cause);

            return rejection_response_from(&state, &req).into_response();
        }
    }

    next.run(req).await
}

fn rejection_response_from<B>(state: &AppState, req: &http::Request<B>) -> impl IntoResponse {
    let domain_name = &state.config.domain_name;

    // Heroku should always set this header
    let request_id = req
        .headers()
        .get("x-request-id")
        .map(|val| val.to_str().unwrap_or_default())
        .unwrap_or_default();

    let body = format!(
        "We are unable to process your request at this time. \
                 This usually means that you are in violation of our crawler \
                 policy (https://{domain_name}/policies#crawlers). \
                 Please open an issue at https://github.com/rust-lang/crates.io \
                 or email help@crates.io \
                 and provide the request id {request_id}"
    );

    (StatusCode::FORBIDDEN, body).into_response()
}

/// Allow blocking individual routes by their pattern through the `BLOCKED_ROUTES`
/// environment variable.
pub async fn block_routes<B>(
    matched_path: Option<MatchedPath>,
    state: AppState,
    req: http::Request<B>,
    next: Next<B>,
) -> axum::response::Response {
    if let Some(matched_path) = matched_path {
        if state.config.blocked_routes.contains(matched_path.as_str()) {
            return RouteBlocked.into_response();
        }
    }

    next.run(req).await
}
