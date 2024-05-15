use crate::app::AppState;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::real_ip::RealIp;
use crate::util::errors::custom;
use axum::extract::{Extension, MatchedPath, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{HeaderMap, StatusCode};

pub async fn middleware(
    Extension(real_ip): Extension<RealIp>,
    matched_path: Option<MatchedPath>,
    state: AppState,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, Response> {
    block_by_ip(&real_ip, &state, req.headers())?;
    block_by_header(&state, &req)?;
    block_routes(matched_path.as_ref(), &state)?;

    Ok(next.run(req).await)
}

/// Middleware that blocks requests if a header matches the given list
///
/// To use, set the `BLOCKED_TRAFFIC` environment variable to a comma-separated list of pairs
/// containing a header name, an equals sign, and the name of another environment variable that
/// contains the values of that header that should be blocked. For example, set `BLOCKED_TRAFFIC`
/// to `User-Agent=BLOCKED_UAS` and `BLOCKED_UAS` to `curl/7.54.0,cargo 1.36.0 (c4fcfb725 2019-05-15)`
/// to block requests from the versions of curl or Cargo specified (values are nonsensical examples).
/// Values of the headers must match exactly.
pub fn block_by_header(state: &AppState, req: &Request) -> Result<(), Response> {
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

            return Err(rejection_response_from(state, req.headers()));
        }
    }

    Ok(())
}

pub fn block_by_ip(
    real_ip: &RealIp,
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), Response> {
    if state.config.blocked_ips.contains(real_ip) {
        return Err(rejection_response_from(state, headers));
    }

    Ok(())
}

fn rejection_response_from(state: &AppState, headers: &HeaderMap) -> Response {
    let domain_name = &state.config.domain_name;

    // Heroku should always set this header
    let request_id = headers
        .get("x-request-id")
        .map(|val| val.to_str().unwrap_or_default())
        .unwrap_or_default();

    let body = format!(
        "We are unable to process your request at this time. \
         This usually means that you are in violation of our API data access \
         policy (https://{domain_name}/data-access). \
         Please email help@crates.io and provide the request id {request_id}"
    );

    (StatusCode::FORBIDDEN, body).into_response()
}

/// Allow blocking individual routes by their pattern through the `BLOCKED_ROUTES`
/// environment variable.
pub fn block_routes(matched_path: Option<&MatchedPath>, state: &AppState) -> Result<(), Response> {
    if let Some(matched_path) = matched_path {
        if state.config.blocked_routes.contains(matched_path.as_str()) {
            let body = "This route is temporarily blocked. See https://status.crates.io.";
            let error = custom(StatusCode::SERVICE_UNAVAILABLE, body);
            return Err(error.into_response());
        }
    }

    Ok(())
}
