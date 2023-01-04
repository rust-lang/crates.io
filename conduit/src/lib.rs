#![warn(rust_2018_idioms)]

use bytes::Bytes;
use std::error::Error;
use std::io::Cursor;

pub use http::{header, Extensions, HeaderMap, Method, Request, Response, StatusCode, Uri};

pub type ConduitRequest = Request<Cursor<Bytes>>;
pub type ResponseResult<Error> = Result<Response<Bytes>, Error>;

pub type BoxError = Box<dyn Error + Send>;
pub type HandlerResult = Result<Response<Bytes>, BoxError>;

/// A helper to convert a concrete error type into a `Box<dyn Error + Send>`
///
/// # Example
///
/// ```
/// # use std::error::Error;
/// # use bytes::Bytes;
/// # use conduit::{box_error, Response};
/// # let _: Result<Response<Bytes>, Box<dyn Error + Send>> =
/// Response::builder().body(Bytes::new()).map_err(box_error);
/// ```
pub fn box_error<E: Error + Send + 'static>(error: E) -> BoxError {
    Box::new(error)
}

pub trait RequestExt {
    /// The byte-size of the body, if any
    fn content_length(&self) -> Option<u64>;
}

impl RequestExt for ConduitRequest {
    fn content_length(&self) -> Option<u64> {
        Some(self.body().get_ref().len() as u64)
    }
}

/// A Handler takes a request and returns a response or an error.
/// By default, a bare function implements `Handler`.
pub trait Handler: Sync + Send + 'static {
    fn call(&self, request: &mut ConduitRequest) -> HandlerResult;
}

impl<F, E> Handler for F
where
    F: Fn(&mut ConduitRequest) -> ResponseResult<E> + Sync + Send + 'static,
    E: Error + Send + 'static,
{
    fn call(&self, request: &mut ConduitRequest) -> HandlerResult {
        (*self)(request).map_err(box_error)
    }
}
