//! Middleware that blocks requests with no user-agent header
//!
//! CloudFront synthesizes `Amazon CloudFront` when the viewer request has no `User-Agent`, so this
//! middleware treats that value as equivalent to a missing header.
//!
//! Requests to the download endpoint are always allowed, to support versions of cargo older than
//! 0.17 (released alongside rustc 1.17).

use crate::middleware::log_request::RequestLogExt;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::UserAgent;
use http::StatusCode;

const CLOUDFRONT_USER_AGENT: &str = "Amazon CloudFront";

pub async fn require_user_agent(
    user_agent: Option<TypedHeader<UserAgent>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let agent = match user_agent {
        Some(ref header) => header.as_str(),
        None => "",
    };

    let has_user_agent = !agent.is_empty() && agent != CLOUDFRONT_USER_AGENT;
    let is_download = req.uri().path().ends_with("download");

    if !has_user_agent && !is_download {
        req.request_log().add("cause", "no user agent");

        let request_id = req
            .headers()
            .get("x-request-id")
            .map(|header| header.to_str().unwrap_or_default())
            .unwrap_or_default();

        let body = format!(include_str!("no_user_agent_message.txt"), request_id);

        (StatusCode::FORBIDDEN, body).into_response()
    } else {
        next.run(req).await
    }
}
