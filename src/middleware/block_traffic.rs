use crate::app::AppState;
use crate::middleware::log_request::RequestLogExt;
use crate::middleware::real_ip::RealIp;
use crate::util::errors::{BoxedAppError, custom};
use axum::extract::{Extension, MatchedPath, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{HeaderMap, StatusCode};
use regex::Regex;

pub async fn middleware(
    Extension(real_ip): Extension<RealIp>,
    matched_path: Option<MatchedPath>,
    state: AppState,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, Response> {
    block_by_ip(&real_ip, &state, req.headers()).map_err(IntoResponse::into_response)?;
    block_by_header(&state, &req).map_err(IntoResponse::into_response)?;
    block_routes(matched_path.as_ref(), &state).map_err(IntoResponse::into_response)?;

    Ok(next.run(req).await)
}

#[derive(Debug)]
pub enum BlockCriteria {
    Regex(Regex),
    String(String),
}

impl BlockCriteria {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Regex(r) => r.is_match(value),
            Self::String(s) => s == value,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Regex(r) => r.as_str(),
            Self::String(s) => s,
        }
    }
}

impl TryFrom<&str> for BlockCriteria {
    type Error = regex::Error;

    /// Parse a string into a [`BlockCriteria`].
    ///
    /// - If the specified string starts and ends with `/` and has at least one character between
    ///   the slashes, interpret the value as a [`Regex`].
    /// - Otherwise, interpret the value as an exact equality match.
    ///
    /// Returns `Err` if the value is interpreted as a regex but does not parse as one.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let is_regex = s.starts_with('/') && s.ends_with('/') && s.len() > 2;
        if is_regex {
            // Slicing is safe here because we checked the starting and ending characters and the
            // length before entering this branch
            Ok(Self::Regex(Regex::new(&s[1..s.len() - 1])?))
        } else {
            Ok(Self::String(s.into()))
        }
    }
}

/// Middleware that blocks requests if a header matches the given criteria list
///
/// To use, set the `BLOCKED_TRAFFIC` environment variable to a comma-separated list of pairs
/// containing a header name, an equals sign, and the name of another environment variable that
/// contains the regex pattern or string values of that header that should be blocked.
///
/// For example, set `BLOCKED_TRAFFIC` to `User-Agent=BLOCKED_UAS` and `BLOCKED_UAS` to
/// `/curl\/[\d]+\.[\d]+\.[\d]+/,cargo 1.36.0 (c4fcfb725 2019-05-15)` to block requests from any
/// version of curl and the exact version of Cargo specified (values are nonsensical examples).
///
/// Values of the headers must start and end with `/` to be interpreted as a regex. Values
/// interpreted as strings must match exactly, in full.
pub fn block_by_header(state: &AppState, req: &Request) -> Result<(), impl IntoResponse> {
    let blocked_traffic = &state.config.block.traffic;

    for (header_name, blocked_values) in blocked_traffic {
        let has_blocked_value = req.headers().get_all(header_name).iter().any(|value| {
            value
                .to_str()
                .map(|ascii_val| blocked_values.iter().any(|v| v.matches(ascii_val)))
                .unwrap_or(false)
        });
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
) -> Result<(), impl IntoResponse> {
    if state.config.block.ips.contains(real_ip) {
        return Err(rejection_response_from(state, headers));
    }

    Ok(())
}

fn rejection_response_from(state: &AppState, headers: &HeaderMap) -> impl IntoResponse {
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

    (StatusCode::FORBIDDEN, body)
}

/// Allow blocking individual routes by their pattern through the `BLOCKED_ROUTES`
/// environment variable.
pub fn block_routes(
    matched_path: Option<&MatchedPath>,
    state: &AppState,
) -> Result<(), BoxedAppError> {
    if let Some(matched_path) = matched_path
        && state.config.block.routes.contains(matched_path.as_str())
    {
        let body = "This route is temporarily blocked. See https://status.crates.io.";
        return Err(custom(StatusCode::SERVICE_UNAVAILABLE, body));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_compact_debug_snapshot;

    #[test]
    fn try_from_str() {
        assert_compact_debug_snapshot!(BlockCriteria::try_from("/"), @r#"Ok(String("/"))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("//"), @r#"Ok(String("//"))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("/hello i am not regex"), @r#"Ok(String("/hello i am not regex"))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("hello me neither//"), @r#"Ok(String("hello me neither//"))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("+"), @r#"Ok(String("+"))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("/yes this is regex/"), @r#"Ok(Regex(Regex("yes this is regex")))"#);
        assert_compact_debug_snapshot!(BlockCriteria::try_from("/)/"), @"
        Err(Syntax(
        ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        regex parse error:
            )
            ^
        error: unopened group
        ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        ))
        ");
    }
}
