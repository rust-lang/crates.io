use axum::async_trait;
use axum::body::Bytes;
use axum::extract::FromRequest;
use hyper::Body;
use std::error::Error;
use std::io::Cursor;
use std::ops::{Deref, DerefMut};

use crate::fallback::check_content_length;
use crate::response::AxumResponse;
use crate::server_error_response;
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

#[async_trait]
impl<S> FromRequest<S, Body> for ConduitRequest
where
    S: Send + Sync,
{
    type Rejection = AxumResponse;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        check_content_length(&req)?;

        let (parts, body) = req.into_parts();

        let full_body = match hyper::body::to_bytes(body).await {
            Ok(body) => body,
            Err(err) => return Err(server_error_response(&err)),
        };

        let request = Request::from_parts(parts, Cursor::new(full_body));
        Ok(ConduitRequest(request))
    }
}
