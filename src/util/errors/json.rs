use std::fmt;

use super::{AppError, InternalAppErrorStatic};
use crate::util::{json_response, AppResponse};

use chrono::NaiveDateTime;
use conduit::{header, StatusCode};

/// Generates a response with the provided status and description as JSON
fn json_error(detail: &str, status: StatusCode) -> AppResponse {
    let json = json!({ "errors": [{ "detail": detail }] });

    let mut response = json_response(&json);
    *response.status_mut() = status;
    response
}

// The following structs are emtpy and do not provide a custom message to the user

#[derive(Debug)]
pub(crate) struct NotFound;

// This struct has this helper impl for use as `NotFound.into()`
impl From<NotFound> for AppResponse {
    fn from(_: NotFound) -> AppResponse {
        json_error("Not Found", StatusCode::NOT_FOUND)
    }
}

impl AppError for NotFound {
    fn response(&self) -> Option<AppResponse> {
        Some(Self.into())
    }
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Not Found".fmt(f)
    }
}

#[derive(Debug)]
pub(super) struct Forbidden;
#[derive(Debug)]
pub(crate) struct ReadOnlyMode;

impl AppError for Forbidden {
    fn response(&self) -> Option<AppResponse> {
        let detail = "must be logged in to perform that action";
        Some(json_error(detail, StatusCode::FORBIDDEN))
    }
}

impl fmt::Display for Forbidden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "must be logged in to perform that action".fmt(f)
    }
}

impl AppError for ReadOnlyMode {
    fn response(&self) -> Option<AppResponse> {
        let detail = "Crates.io is currently in read-only mode for maintenance. \
                      Please try again later.";
        Some(json_error(detail, StatusCode::SERVICE_UNAVAILABLE))
    }
}

impl fmt::Display for ReadOnlyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Tried to write in read only mode".fmt(f)
    }
}

// The following structs wrap owned data and provide a custom message to the user

#[derive(Debug)]
pub(super) struct Ok(pub(super) String);
#[derive(Debug)]
pub(super) struct BadRequest(pub(super) String);
#[derive(Debug)]
pub(super) struct ServerError(pub(super) String);
#[derive(Debug)]
pub(crate) struct ServiceUnavailable(pub(super) String);
#[derive(Debug)]
pub(crate) struct TooManyRequests {
    pub retry_after: NaiveDateTime,
}

impl AppError for Ok {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::OK))
    }
}

impl fmt::Display for Ok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for BadRequest {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::BAD_REQUEST))
    }
}

impl fmt::Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for ServerError {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for ServiceUnavailable {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::SERVICE_UNAVAILABLE))
    }
}

impl fmt::Display for ServiceUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for TooManyRequests {
    fn response(&self) -> Option<AppResponse> {
        const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
        let retry_after = self.retry_after.format(HTTP_DATE_FORMAT);

        let detail = format!(
            "You have published too many crates in a \
             short period of time. Please try again after {retry_after} or email \
             help@crates.io to have your limit increased."
        );
        let mut response = json_error(&detail, StatusCode::TOO_MANY_REQUESTS);
        response.headers_mut().insert(
            header::RETRY_AFTER,
            retry_after
                .to_string()
                .try_into()
                .expect("HTTP_DATE_FORMAT contains invalid char"),
        );
        Some(response)
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
    pub fn boxed() -> Box<dyn AppError> {
        Box::new(Self)
    }
}

impl AppError for InsecurelyGeneratedTokenRevoked {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.to_string(), StatusCode::UNAUTHORIZED))
    }

    fn cause(&self) -> Option<&dyn AppError> {
        Some(&InternalAppErrorStatic {
            description: "insecurely generated, revoked 2020-07",
        })
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

#[derive(Debug)]
pub(super) struct AccountLocked {
    pub(super) reason: String,
    pub(super) until: Option<NaiveDateTime>,
}

impl AppError for AccountLocked {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.to_string(), StatusCode::FORBIDDEN))
    }
}

impl fmt::Display for AccountLocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(until) = self.until {
            let until = until.format("%Y-%m-%d at %H:%M:%S UTC");
            write!(
                f,
                "This account is locked until {}. Reason: {}",
                until, self.reason
            )
        } else {
            write!(
                f,
                "This account is indefinitely locked. Reason: {}",
                self.reason
            )
        }
    }
}

#[derive(Debug)]
pub(crate) struct OwnershipInvitationExpired {
    pub(crate) crate_name: String,
}

impl AppError for OwnershipInvitationExpired {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.to_string(), StatusCode::GONE))
    }
}

impl fmt::Display for OwnershipInvitationExpired {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The invitation to become an owner of the {} crate expired. \
             Please reach out to an owner of the crate to request a new invitation.",
            self.crate_name
        )
    }
}

#[derive(Debug)]
pub(crate) struct MetricsDisabled;

impl AppError for MetricsDisabled {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.to_string(), StatusCode::NOT_FOUND))
    }
}

impl fmt::Display for MetricsDisabled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Metrics are disabled on this crates.io instance")
    }
}

#[derive(Debug)]
pub(crate) struct RouteBlocked;

impl AppError for RouteBlocked {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(
            &self.to_string(),
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

impl fmt::Display for RouteBlocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("This route is temporarily blocked. See https://status.crates.io.")
    }
}
