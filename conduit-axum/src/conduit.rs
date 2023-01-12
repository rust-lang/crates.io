use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::response::IntoResponse;
use axum::{async_trait, RequestExt};
use http_body::LengthLimitError;
use hyper::Body;
use std::error::Error;
use std::ops::{Deref, DerefMut};

use crate::server_error_response;
pub use http::{header, Extensions, HeaderMap, Method, Request, Response, StatusCode, Uri};

pub type BoxError = Box<dyn Error + Send>;

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
pub struct BytesRequest(pub Request<Bytes>);

impl Deref for BytesRequest {
    type Target = Request<Bytes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BytesRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl<S> FromRequest<S, Body> for BytesRequest
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let request = match req.with_limited_body() {
            Ok(req) => {
                let (parts, body) = req.into_parts();

                let bytes = hyper::body::to_bytes(body).await.map_err(|err| {
                    if err.downcast_ref::<LengthLimitError>().is_some() {
                        StatusCode::BAD_REQUEST.into_response()
                    } else {
                        server_error_response(&*err)
                    }
                })?;

                Request::from_parts(parts, bytes)
            }
            Err(req) => {
                let (parts, body) = req.into_parts();

                let bytes = hyper::body::to_bytes(body)
                    .await
                    .map_err(|err| server_error_response(&err))?;

                Request::from_parts(parts, bytes)
            }
        };

        Ok(BytesRequest(request))
    }
}
