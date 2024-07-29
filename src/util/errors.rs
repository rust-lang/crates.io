//! This module implements several error types and traits.
//!
//! The suggested usage in returned results is as follows:
//!
//! * The concrete `util::concrete::Error` type (re-exported as `util::Error`) is great for code
//!   that is not part of the request/response lifecycle.  It avoids pulling in the unnecessary
//!   infrastructure to convert errors into a user facing JSON responses (relative to `AppError`).
//! * `diesel::QueryResult` - There is a lot of code that only deals with query errors.  If only
//!   one type of error is possible in a function, using that specific error is preferable to the
//!   more general `util::Error`.  This is especially common in model code.
//! * `util::errors::AppResult` - Some failures should be converted into user facing JSON
//!   responses.  This error type is more dynamic and is box allocated.  Low-level errors are
//!   typically not converted to user facing errors and most usage is within the models,
//!   controllers, and middleware layers.

use axum::response::IntoResponse;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

use axum::Extension;
use chrono::NaiveDateTime;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use http::StatusCode;
use tokio::task::JoinError;

use crate::middleware::log_request::ErrorField;

mod json;

use crate::email::EmailError;
use crates_io_github::GitHubError;
pub use json::TOKEN_FORMAT_ERROR;
pub(crate) use json::{custom, InsecurelyGeneratedTokenRevoked, ReadOnlyMode, TooManyRequests};

pub type BoxedAppError = Box<dyn AppError>;

// The following are intended to be used for errors being sent back to the Ember
// frontend, not to cargo as cargo does not handle non-200 response codes well
// (see <https://github.com/rust-lang/cargo/issues/3995>), but Ember requires
// non-200 response codes for its stores to work properly.

/// Return an error with status 400 and the provided description as JSON
pub fn bad_request<S: ToString>(error: S) -> BoxedAppError {
    custom(StatusCode::BAD_REQUEST, error.to_string())
}

pub fn account_locked(reason: &str, until: Option<NaiveDateTime>) -> BoxedAppError {
    let detail = until
        .map(|until| until.format("%Y-%m-%d at %H:%M:%S UTC"))
        .map(|until| format!("This account is locked until {until}. Reason: {reason}"))
        .unwrap_or_else(|| format!("This account is indefinitely locked. Reason: {reason}"));

    custom(StatusCode::FORBIDDEN, detail)
}

pub fn forbidden(detail: impl Into<Cow<'static, str>>) -> BoxedAppError {
    custom(StatusCode::FORBIDDEN, detail)
}

pub fn not_found() -> BoxedAppError {
    custom(StatusCode::NOT_FOUND, "Not Found")
}

/// Returns an error with status 500 and the provided description as JSON
pub fn server_error<S: ToString>(error: S) -> BoxedAppError {
    custom(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

/// Returns an error with status 503 and the provided description as JSON
pub fn service_unavailable() -> BoxedAppError {
    custom(StatusCode::SERVICE_UNAVAILABLE, "Service unavailable")
}

pub fn crate_not_found(krate: &str) -> BoxedAppError {
    let detail = format!("crate `{krate}` does not exist");
    custom(StatusCode::NOT_FOUND, detail)
}

pub fn version_not_found(krate: &str, version: &str) -> BoxedAppError {
    let detail = format!("crate `{krate}` does not have a version `{version}`");
    custom(StatusCode::NOT_FOUND, detail)
}

// =============================================================================
// AppError trait

pub trait AppError: Send + fmt::Display + fmt::Debug + 'static {
    /// Generate an HTTP response for the error
    ///
    /// If none is returned, the error will bubble up the middleware stack
    /// where it is eventually logged and turned into a status 500 response.
    fn response(&self) -> axum::response::Response;

    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl dyn AppError {
    pub fn is<T: Any>(&self) -> bool {
        self.get_type_id() == TypeId::of::<T>()
    }
}

impl AppError for BoxedAppError {
    fn response(&self) -> axum::response::Response {
        (**self).response()
    }

    fn get_type_id(&self) -> TypeId {
        (**self).get_type_id()
    }
}

impl IntoResponse for BoxedAppError {
    fn into_response(self) -> axum::response::Response {
        self.response()
    }
}

pub type AppResult<T> = Result<T, BoxedAppError>;

// =============================================================================
// Error impls

impl<E: Error + Send + 'static> AppError for E {
    fn response(&self) -> axum::response::Response {
        error!(error = %self, "Internal Server Error");

        sentry::capture_error(self);

        server_error_response(self.to_string())
    }
}

impl From<diesel::ConnectionError> for BoxedAppError {
    fn from(err: diesel::ConnectionError) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<DieselError> for BoxedAppError {
    fn from(err: DieselError) -> BoxedAppError {
        match err {
            DieselError::NotFound => not_found(),
            DieselError::DatabaseError(_, info)
                if info.message().ends_with("read-only transaction") =>
            {
                Box::new(ReadOnlyMode)
            }
            DieselError::DatabaseError(DatabaseErrorKind::ClosedConnection, _) => {
                service_unavailable()
            }
            _ => Box::new(err),
        }
    }
}

impl From<EmailError> for BoxedAppError {
    fn from(error: EmailError) -> Self {
        match error {
            EmailError::AddressError(error) => Box::new(error),
            EmailError::MessageBuilderError(error) => Box::new(error),
            EmailError::TransportError(error) => {
                error!(?error, "Failed to send email");
                server_error("Failed to send the email")
            }
        }
    }
}

impl From<diesel_async::pooled_connection::deadpool::PoolError> for BoxedAppError {
    fn from(err: diesel_async::pooled_connection::deadpool::PoolError) -> BoxedAppError {
        error!("Database pool error: {err}");
        service_unavailable()
    }
}

impl From<prometheus::Error> for BoxedAppError {
    fn from(err: prometheus::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<reqwest::Error> for BoxedAppError {
    fn from(err: reqwest::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<serde_json::Error> for BoxedAppError {
    fn from(err: serde_json::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<std::io::Error> for BoxedAppError {
    fn from(err: std::io::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<crates_io_worker::EnqueueError> for BoxedAppError {
    fn from(err: crates_io_worker::EnqueueError) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<JoinError> for BoxedAppError {
    fn from(err: JoinError) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<GitHubError> for BoxedAppError {
    fn from(error: GitHubError) -> Self {
        match error {
            GitHubError::Permission(_) => custom(
                StatusCode::FORBIDDEN,
                "It looks like you don't have permission \
                     to query a necessary property from GitHub \
                     to complete this request. \
                     You may need to re-authenticate on \
                     crates.io to grant permission to read \
                     GitHub org memberships.",
            ),
            GitHubError::NotFound(_) => not_found(),
            _ => internal(format!("didn't get a 200 result from github: {error}")),
        }
    }
}

// =============================================================================
// Internal error for use with `chain_error`

#[derive(Debug)]
struct InternalAppError {
    description: String,
}

impl fmt::Display for InternalAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)?;
        Ok(())
    }
}

impl AppError for InternalAppError {
    fn response(&self) -> axum::response::Response {
        error!(error = %self.description, "Internal Server Error");

        sentry::capture_message(&self.description, sentry::Level::Error);

        server_error_response(self.description.to_string())
    }
}

pub fn internal<S: ToString>(error: S) -> BoxedAppError {
    Box::new(InternalAppError {
        description: error.to_string(),
    })
}

fn server_error_response(error: String) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Extension(ErrorField(error)),
        "Internal Server Error",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::result::Error as DieselError;
    use http::StatusCode;

    #[test]
    fn http_error_responses() {
        use crate::serde::de::Error;

        // Types for handling common error status codes
        assert_eq!(bad_request("").response().status(), StatusCode::BAD_REQUEST);
        assert_eq!(forbidden("").response().status(), StatusCode::FORBIDDEN);
        assert_eq!(
            BoxedAppError::from(DieselError::NotFound)
                .response()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(not_found().response().status(), StatusCode::NOT_FOUND);

        // All other error types are converted to internal server errors
        assert_eq!(
            internal("").response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            BoxedAppError::from(serde_json::Error::custom("ExpectedColon"))
                .response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            BoxedAppError::from(::std::io::Error::new(::std::io::ErrorKind::Other, ""))
                .response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
