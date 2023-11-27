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
use std::error::Error;
use std::fmt;

use axum::Extension;
use chrono::NaiveDateTime;
use diesel::result::Error as DieselError;
use http::StatusCode;
use tokio::task::JoinError;

use crate::db::PoolError;
use crate::middleware::log_request::{CauseField, ErrorField};

mod json;

pub use json::TOKEN_FORMAT_ERROR;
pub(crate) use json::{
    InsecurelyGeneratedTokenRevoked, MetricsDisabled, NotFound, OwnershipInvitationExpired,
    ReadOnlyMode, RouteBlocked, TooManyRequests,
};

pub type BoxedAppError = Box<dyn AppError>;

/// Returns an error with status 200 and the provided description as JSON
///
/// This is for backwards compatibility with cargo endpoints.  For all other
/// endpoints, use helpers like `bad_request` or `server_error` which set a
/// correct status code.
pub fn cargo_err<S: ToString>(error: S) -> BoxedAppError {
    Box::new(json::Ok(error.to_string()))
}

// The following are intended to be used for errors being sent back to the Ember
// frontend, not to cargo as cargo does not handle non-200 response codes well
// (see <https://github.com/rust-lang/cargo/issues/3995>), but Ember requires
// non-200 response codes for its stores to work properly.

/// Return an error with status 400 and the provided description as JSON
pub fn bad_request<S: ToString>(error: S) -> BoxedAppError {
    Box::new(json::BadRequest(error.to_string()))
}

pub fn account_locked(reason: &str, until: Option<NaiveDateTime>) -> BoxedAppError {
    Box::new(json::AccountLocked {
        reason: reason.to_string(),
        until,
    })
}

pub fn forbidden() -> BoxedAppError {
    Box::new(json::Forbidden)
}

pub fn not_found() -> BoxedAppError {
    Box::new(json::NotFound)
}

/// Returns an error with status 500 and the provided description as JSON
pub fn server_error<S: ToString>(error: S) -> BoxedAppError {
    Box::new(json::ServerError(error.to_string()))
}

/// Returns an error with status 503 and the provided description as JSON
pub fn service_unavailable<S: ToString>(error: S) -> BoxedAppError {
    Box::new(json::ServiceUnavailable(error.to_string()))
}

// =============================================================================
// AppError trait

pub trait AppError: Send + fmt::Display + fmt::Debug + 'static {
    /// Generate an HTTP response for the error
    ///
    /// If none is returned, the error will bubble up the middleware stack
    /// where it is eventually logged and turned into a status 500 response.
    fn response(&self) -> axum::response::Response;

    /// The cause of an error response
    ///
    /// If present, an error provided to the `LogRequests` middleware.
    ///
    /// This is intended for use with the `ChainError` trait, where a user facing
    /// error may wrap an internal error we still want to log.
    fn cause(&self) -> Option<&dyn AppError> {
        None
    }

    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn chain<E>(self, error: E) -> BoxedAppError
    where
        Self: Sized,
        E: AppError,
    {
        Box::new(ChainedError {
            error,
            cause: Box::new(self),
        })
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

    fn cause(&self) -> Option<&dyn AppError> {
        (**self).cause()
    }

    fn get_type_id(&self) -> TypeId {
        (**self).get_type_id()
    }
}

impl IntoResponse for BoxedAppError {
    fn into_response(self) -> axum::response::Response {
        let mut response = self.response();

        if let Some(cause) = self.cause() {
            response
                .extensions_mut()
                .insert(CauseField(cause.to_string()));
        }

        response
    }
}

pub type AppResult<T> = Result<T, BoxedAppError>;

// =============================================================================
// Chaining errors

#[derive(Debug)]
struct ChainedError<E> {
    error: E,
    cause: BoxedAppError,
}

impl<E: AppError> AppError for ChainedError<E> {
    fn response(&self) -> axum::response::Response {
        self.error.response()
    }

    fn cause(&self) -> Option<&dyn AppError> {
        Some(&*self.cause)
    }
}

impl<E: AppError> fmt::Display for ChainedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} caused by {}", self.error, self.cause)
    }
}

// =============================================================================
// Error impls

impl<E: Error + Send + 'static> AppError for E {
    fn response(&self) -> axum::response::Response {
        error!(error = %self, "Internal Server Error");

        sentry::capture_error(self);

        server_error_response(self.to_string())
    }
}

impl From<base64::DecodeError> for BoxedAppError {
    fn from(err: base64::DecodeError) -> BoxedAppError {
        Box::new(err)
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
            _ => Box::new(err),
        }
    }
}

impl From<http::Error> for BoxedAppError {
    fn from(err: http::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<lettre::error::Error> for BoxedAppError {
    fn from(err: lettre::error::Error) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<lettre::address::AddressError> for BoxedAppError {
    fn from(err: lettre::address::AddressError) -> BoxedAppError {
        Box::new(err)
    }
}

impl From<PoolError> for BoxedAppError {
    fn from(err: PoolError) -> BoxedAppError {
        match err {
            PoolError::UnhealthyPool => service_unavailable("Service unavailable"),
            _ => Box::new(err),
        }
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

#[derive(Debug)]
struct InternalAppErrorStatic {
    description: &'static str,
}

impl fmt::Display for InternalAppErrorStatic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)?;
        Ok(())
    }
}

impl AppError for InternalAppErrorStatic {
    fn response(&self) -> axum::response::Response {
        error!(error = %self.description, "Internal Server Error");

        sentry::capture_message(self.description, sentry::Level::Error);

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

#[test]
fn chain_error_internal() {
    assert_eq!(
        Err::<(), _>(internal("inner"))
            .map_err(|err| err.chain(internal("middle")))
            .map_err(|err| err.chain(internal("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by middle caused by inner"
    );
    assert_eq!(
        Err::<(), _>(internal("inner"))
            .map_err(|err| err.chain(internal("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by inner"
    );

    // Don't do this, the user will see a generic 500 error instead of the intended message
    assert_eq!(
        Err::<(), _>(cargo_err("inner"))
            .map_err(|err| err.chain(internal("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by inner"
    );
    assert_eq!(
        Err::<(), _>(forbidden())
            .map_err(|err| err.chain(internal("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by must be logged in to perform that action"
    );
}

#[test]
fn chain_error_user_facing() {
    // Do this rarely, the user will only see the outer error
    assert_eq!(
        Err::<(), _>(cargo_err("inner"))
            .map_err(|err| err.chain(cargo_err("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by inner" // never logged
    );

    // The outer error is sent as a response to the client.
    // The inner error never bubbles up to the logging middleware
    assert_eq!(
        Err::<(), _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            .map_err(|err| err.chain(cargo_err("outer")))
            .unwrap_err()
            .to_string(),
        "outer caused by permission denied" // never logged
    );
}
