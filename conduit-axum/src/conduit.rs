use axum::body::Bytes;
use std::error::Error;
use std::io::Cursor;
use std::ops::{Deref, DerefMut};

use crate::response::AxumResponse;
pub use http::{header, Extensions, HeaderMap, Method, Request, Response, StatusCode, Uri};

pub type BoxError = Box<dyn Error + Send>;
pub type HandlerResult = AxumResponse;

/// A helper to convert a concrete error type into a `Box<dyn Error + Send>`
///
/// # Example
///
/// ```
/// # use std::error::Error;
/// # use axum::body::Bytes;
/// # use conduit_axum::{box_error, Response};
/// # let _: Result<Response<Bytes>, Box<dyn Error + Send>> =
/// Response::builder().body(Bytes::new()).map_err(box_error);
/// ```
pub fn box_error<E: Error + Send + 'static>(error: E) -> BoxError {
    Box::new(error)
}

#[derive(Debug)]
pub struct ConduitRequest(pub Request<Cursor<Bytes>>);

impl Deref for ConduitRequest {
    type Target = Request<Cursor<Bytes>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ConduitRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A Handler takes a request and returns a response or an error.
/// By default, a bare function implements `Handler`.
pub trait Handler: Sync + Send + 'static {
    fn call(&self, request: ConduitRequest) -> HandlerResult;
}

impl<F> Handler for F
where
    F: Fn(ConduitRequest) -> HandlerResult + Sync + Send + 'static,
{
    fn call(&self, request: ConduitRequest) -> HandlerResult {
        (*self)(request)
    }
}
