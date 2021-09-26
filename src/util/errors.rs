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

use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt;

use chrono::NaiveDateTime;
use diesel::result::Error as DieselError;

use crate::db::PoolError;
use crate::util::AppResponse;

mod json;

pub use json::TOKEN_FORMAT_ERROR;
pub(crate) use json::{
    InsecurelyGeneratedTokenRevoked, MetricsDisabled, NotFound, OwnershipInvitationExpired,
    ReadOnlyMode, RouteBlocked, TooManyRequests,
};

/// Returns an error with status 200 and the provided description as JSON
///
/// This is for backwards compatibility with cargo endpoints.  For all other
/// endpoints, use helpers like `bad_request` or `server_error` which set a
/// correct status code.
pub fn cargo_err<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(json::Ok(error.to_string()))
}

// The following are intended to be used for errors being sent back to the Ember
// frontend, not to cargo as cargo does not handle non-200 response codes well
// (see <https://github.com/rust-lang/cargo/issues/3995>), but Ember requires
// non-200 response codes for its stores to work properly.

/// Return an error with status 400 and the provided description as JSON
pub fn bad_request<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(json::BadRequest(error.to_string()))
}

pub fn account_locked(reason: &str, until: Option<NaiveDateTime>) -> Box<dyn AppError> {
    Box::new(json::AccountLocked {
        reason: reason.to_string(),
        until,
    })
}

pub fn forbidden() -> Box<dyn AppError> {
    Box::new(json::Forbidden)
}

pub fn not_found() -> Box<dyn AppError> {
    Box::new(json::NotFound)
}

/// Returns an error with status 500 and the provided description as JSON
pub fn server_error<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(json::ServerError(error.to_string()))
}

/// Returns an error with status 503 and the provided description as JSON
pub fn service_unavailable<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(json::ServiceUnavailable(error.to_string()))
}

// =============================================================================
// AppError trait

pub trait AppError: Send + fmt::Display + fmt::Debug + 'static {
    /// Generate an HTTP response for the error
    ///
    /// If none is returned, the error will bubble up the middleware stack
    /// where it is eventually logged and turned into a status 500 response.
    fn response(&self) -> Option<AppResponse>;

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

    fn chain<E>(self, error: E) -> Box<dyn AppError>
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

    fn try_convert(err: &(dyn Error + Send + 'static)) -> Option<Box<Self>> {
        if matches!(err.downcast_ref(), Some(PoolError::UnhealthyPool)) {
            return Some(service_unavailable("Service unavailable"));
        }

        match err.downcast_ref() {
            Some(DieselError::NotFound) => Some(not_found()),
            Some(DieselError::DatabaseError(_, info))
                if info.message().ends_with("read-only transaction") =>
            {
                Some(Box::new(ReadOnlyMode))
            }
            _ => None,
        }
    }
}

impl AppError for Box<dyn AppError> {
    fn response(&self) -> Option<AppResponse> {
        (**self).response()
    }

    fn cause(&self) -> Option<&dyn AppError> {
        (**self).cause()
    }

    fn get_type_id(&self) -> TypeId {
        (**self).get_type_id()
    }
}

pub type AppResult<T> = Result<T, Box<dyn AppError>>;

// =============================================================================
// Chaining errors

#[derive(Debug)]
struct ChainedError<E> {
    error: E,
    cause: Box<dyn AppError>,
}

impl<E: AppError> AppError for ChainedError<E> {
    fn response(&self) -> Option<AppResponse> {
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
    fn response(&self) -> Option<AppResponse> {
        None
    }
}

impl<E: Error + Send + 'static> From<E> for Box<dyn AppError> {
    fn from(err: E) -> Box<dyn AppError> {
        <dyn AppError>::try_convert(&err).unwrap_or_else(|| Box::new(err))
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
    fn response(&self) -> Option<AppResponse> {
        None
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
    fn response(&self) -> Option<AppResponse> {
        None
    }
}

pub fn internal<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(InternalAppError {
        description: error.to_string(),
    })
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct AppErrToStdErr(pub Box<dyn AppError>);

pub(crate) fn std_error(e: Box<dyn AppError>) -> Box<dyn Error + Send> {
    Box::new(AppErrToStdErr(e))
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
