//! Middleware that blocks requests with no user-agent header
//!
//! By default the middleware will treat "" and "Amazon CloudFront" as a missing user-agent. To
//! change the 2nd value, set `WEB_CDN_USER_AGENT` to the appropriate string. To disable the CDN
//! check, set `WEB_CDN_USER_AGENT` to the empty string.
//!
//! Requests to the download endpoint are always allowed, to support versions of cargo older than
//! 0.17 (released alongside rustc 1.17).

use crate::app::AppState;
use crate::middleware::log_request::RequestLogExt;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::UserAgent;
use http::StatusCode;

pub async fn require_user_agent(
    user_agent: Option<TypedHeader<UserAgent>>,
    state: AppState,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let cdn_user_agent = &state.config.cdn_user_agent;

    let agent = match user_agent {
        Some(ref header) => header.as_str(),
        None => "",
    };

    let has_user_agent = !agent.is_empty() && agent != cdn_user_agent;
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
