use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum_extra::json;
use std::borrow::Cow;
use std::fmt;

use super::{AppError, BoxedAppError};

use crate::middleware::log_request::CauseField;
use crate::rate_limiter::LimitedAction;
use chrono::NaiveDateTime;
use http::{header, StatusCode};

/// Generates a response with the provided status and description as JSON
fn json_error(detail: &str, status: StatusCode) -> Response {
    let json = json!({ "errors": [{ "detail": detail }] });
    (status, json).into_response()
}

// The following structs wrap owned data and provide a custom message to the user

pub fn custom(status: StatusCode, detail: impl Into<Cow<'static, str>>) -> BoxedAppError {
    Box::new(CustomApiError {
        status,
        detail: detail.into(),
    })
}

#[derive(Debug, Clone)]
pub struct CustomApiError {
    status: StatusCode,
    detail: Cow<'static, str>,
}

impl fmt::Display for CustomApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.detail.fmt(f)
    }
}

impl AppError for CustomApiError {
    fn response(&self) -> Response {
        json_error(&self.detail, self.status)
    }
}

#[derive(Debug)]
pub(crate) struct TooManyRequests {
    pub action: LimitedAction,
    pub retry_after: NaiveDateTime,
}

impl AppError for TooManyRequests {
    fn response(&self) -> Response {
        const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %T GMT";
        let retry_after = self.retry_after.format(HTTP_DATE_FORMAT);

        let detail = format!(
            "{}. Please try again after {retry_after} or email \
             help@crates.io to have your limit increased.",
            self.action.error_message()
        );
        let mut response = json_error(&detail, StatusCode::TOO_MANY_REQUESTS);
        response.headers_mut().insert(
            header::RETRY_AFTER,
            retry_after
                .to_string()
                .try_into()
                .expect("HTTP_DATE_FORMAT contains invalid char"),
        );
        response
    }
}

impl fmt::Display for TooManyRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Too many requests".fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InsecurelyGeneratedTokenRevoked;

impl InsecurelyGeneratedTokenRevoked {
    pub fn boxed() -> BoxedAppError {
        Box::new(Self)
    }
}

impl AppError for InsecurelyGeneratedTokenRevoked {
    fn response(&self) -> Response {
        let cause = CauseField("insecurely generated, revoked 2020-07".to_string());
        let response = json_error(&self.to_string(), StatusCode::UNAUTHORIZED);
        (Extension(cause), response).into_response()
    }
}

pub const TOKEN_FORMAT_ERROR: &str =
    "The given API token does not match the format used by crates.io. \
    \
    Tokens generated before 2020-07-14 were generated with an insecure \
    random number generator, and have been revoked. You can generate a \
    new token at https://crates.io/me. \
    \
    For more information please see \
    https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. \
    We apologize for any inconvenience.";

impl fmt::Display for InsecurelyGeneratedTokenRevoked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(TOKEN_FORMAT_ERROR)?;
        Result::Ok(())
    }
}
