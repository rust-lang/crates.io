use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt;

use chrono::NaiveDateTime;
use conduit::Response;
use diesel::result::Error as DieselError;

use crate::util::json_response;

mod http;

/// Returns an error with status 200 and the provided description as JSON
///
/// This is for backwards compatibility with cargo endpoints.  For all other
/// endpoints, use helpers like `bad_request` or `server_error` which set a
/// correct status code.
pub fn cargo_err<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(http::Ok(error.to_string()))
}

// The following are intended to be used for errors being sent back to the Ember
// frontend, not to cargo as cargo does not handle non-200 response codes well
// (see <https://github.com/rust-lang/cargo/issues/3995>), but Ember requires
// non-200 response codes for its stores to work properly.

/// Returns an error with status 500 and the provided description as JSON
pub fn server_error<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(http::ServerError(error.to_string()))
}

#[derive(Serialize)]
struct StringError {
    detail: String,
}
#[derive(Serialize)]
struct Bad {
    errors: Vec<StringError>,
}

// =============================================================================
// AppError trait

pub trait AppError: Send + fmt::Display + fmt::Debug + 'static {
    /// Generate an HTTP response for the error
    ///
    /// If none is returned, the error will bubble up the middleware stack
    /// where it is eventually logged and turned into a status 500 response.
    fn response(&self) -> Option<Response>;

    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl dyn AppError {
    pub fn is<T: Any>(&self) -> bool {
        self.get_type_id() == TypeId::of::<T>()
    }

    pub fn from_std_error(err: Box<dyn Error + Send>) -> Box<dyn AppError> {
        Self::try_convert(&*err).unwrap_or_else(|| internal(&err))
    }

    fn try_convert(err: &(dyn Error + Send + 'static)) -> Option<Box<Self>> {
        match err.downcast_ref() {
            Some(DieselError::NotFound) => Some(Box::new(NotFound)),
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
    fn response(&self) -> Option<Response> {
        (**self).response()
    }
}

pub type AppResult<T> = Result<T, Box<dyn AppError>>;

// =============================================================================
// Chaining errors

pub trait ChainError<T> {
    fn chain_error<E, F>(self, callback: F) -> AppResult<T>
    where
        E: AppError,
        F: FnOnce() -> E;
}

#[derive(Debug)]
struct ChainedError<E> {
    error: E,
    cause: Box<dyn AppError>,
}

impl<T, F> ChainError<T> for F
where
    F: FnOnce() -> AppResult<T>,
{
    fn chain_error<E, C>(self, callback: C) -> AppResult<T>
    where
        E: AppError,
        C: FnOnce() -> E,
    {
        self().chain_error(callback)
    }
}

impl<T, E: AppError> ChainError<T> for Result<T, E> {
    fn chain_error<E2, C>(self, callback: C) -> AppResult<T>
    where
        E2: AppError,
        C: FnOnce() -> E2,
    {
        self.map_err(move |err| {
            Box::new(ChainedError {
                error: callback(),
                cause: Box::new(err),
            }) as Box<dyn AppError>
        })
    }
}

impl<T> ChainError<T> for Option<T> {
    fn chain_error<E, C>(self, callback: C) -> AppResult<T>
    where
        E: AppError,
        C: FnOnce() -> E,
    {
        match self {
            Some(t) => Ok(t),
            None => Err(Box::new(callback())),
        }
    }
}

impl<E: AppError> AppError for ChainedError<E> {
    fn response(&self) -> Option<Response> {
        self.error.response()
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
    fn response(&self) -> Option<Response> {
        None
    }
}

impl<E: Error + Send + 'static> From<E> for Box<dyn AppError> {
    fn from(err: E) -> Box<dyn AppError> {
        AppError::try_convert(&err).unwrap_or_else(|| Box::new(err))
    }
}
// =============================================================================
// Concrete errors

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
    fn response(&self) -> Option<Response> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NotFound;

impl AppError for NotFound {
    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: "Not Found".to_string(),
            }],
        });
        response.status = (404, "Not Found");
        Some(response)
    }
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Not Found".fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Unauthorized;

impl AppError for Unauthorized {
    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: "must be logged in to perform that action".to_string(),
            }],
        });
        response.status = (403, "Forbidden");
        Some(response)
    }
}

impl fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "must be logged in to perform that action".fmt(f)
    }
}

#[derive(Debug)]
struct BadRequest(String);

impl AppError for BadRequest {
    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: self.0.clone(),
            }],
        });
        response.status = (400, "Bad Request");
        Some(response)
    }
}

impl fmt::Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub fn internal<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(InternalAppError {
        description: error.to_string(),
    })
}

/// This is intended to be used for errors being sent back to the Ember
/// frontend, not to cargo as cargo does not handle non-200 response codes well
/// (see <https://github.com/rust-lang/cargo/issues/3995>), but Ember requires
/// non-200 response codes for its stores to work properly.
///
/// Since this is going back to the UI these errors are treated the same as
/// `cargo_err` errors, other than the HTTP status code.
pub fn bad_request<S: ToString + ?Sized>(error: &S) -> Box<dyn AppError> {
    Box::new(BadRequest(error.to_string()))
}

#[derive(Debug)]
pub struct AppErrToStdErr(pub Box<dyn AppError>);

impl Error for AppErrToStdErr {}

impl fmt::Display for AppErrToStdErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) fn std_error(e: Box<dyn AppError>) -> Box<dyn Error + Send> {
    Box::new(AppErrToStdErr(e))
}

pub(crate) fn std_error_no_send(e: Box<dyn AppError>) -> Box<dyn Error> {
    Box::new(AppErrToStdErr(e))
}

#[derive(Debug, Clone, Copy)]
pub struct ReadOnlyMode;

impl AppError for ReadOnlyMode {
    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: "Crates.io is currently in read-only mode for maintenance. \
                         Please try again later."
                    .to_string(),
            }],
        });
        response.status = (503, "Service Unavailable");
        Some(response)
    }
}

impl fmt::Display for ReadOnlyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Tried to write in read only mode".fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TooManyRequests {
    pub retry_after: NaiveDateTime,
}

impl AppError for TooManyRequests {
    fn response(&self) -> Option<Response> {
        const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
        let retry_after = self.retry_after.format(HTTP_DATE_FORMAT);

        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: format!(
                    "You have published too many crates in a \
                     short period of time. Please try again after {} or email \
                     help@crates.io to have your limit increased.",
                    retry_after
                ),
            }],
        });
        response.status = (429, "TOO MANY REQUESTS");
        response
            .headers
            .insert("Retry-After".into(), vec![retry_after.to_string()]);
        Some(response)
    }
}

impl fmt::Display for TooManyRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Too many requests".fmt(f)
    }
}

#[test]
fn chain_error_internal() {
    assert_eq!(
        None::<()>
            .chain_error(|| internal("inner"))
            .chain_error(|| internal("middle"))
            .chain_error(|| internal("outer"))
            .unwrap_err()
            .to_string(),
        "outer caused by middle caused by inner"
    );
    assert_eq!(
        Err::<(), _>(internal("inner"))
            .chain_error(|| internal("outer"))
            .unwrap_err()
            .to_string(),
        "outer caused by inner"
    );
}
