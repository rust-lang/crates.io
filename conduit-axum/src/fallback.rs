use crate::adaptor::ConduitRequest;
use crate::error::ServiceError;
use crate::file_stream::FileStream;
use crate::{spawn_blocking, AxumResponse, ConduitResponse};

use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::{Body, HttpBody};
use axum::extract::Extension;
use axum::handler::Handler as AxumHandler;
use axum::response::IntoResponse;
use conduit::{Handler, RequestExt};
use conduit_router::RoutePattern;
use http::header::CONTENT_LENGTH;
use http::StatusCode;
use hyper::{Request, Response};
use tracing::{error, warn};

/// The maximum size allowed in the `Content-Length` header
///
/// Chunked requests may grow to be larger over time if that much data is actually sent.
/// See the usage section of the README if you plan to use this server in production.
const MAX_CONTENT_LENGTH: u64 = 128 * 1024 * 1024; // 128 MB

pub trait ConduitFallback {
    fn conduit_fallback(self, handler: impl Handler) -> Self;
}

impl ConduitFallback for axum::Router {
    fn conduit_fallback(self, handler: impl Handler) -> Self {
        self.fallback(ConduitAxumHandler::wrap(handler))
    }
}

#[derive(Debug)]
pub struct ConduitAxumHandler<H>(pub Arc<H>);

impl<H> ConduitAxumHandler<H> {
    pub fn wrap(handler: H) -> Self {
        Self(Arc::new(handler))
    }
}

impl<H> Clone for ConduitAxumHandler<H> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S, H> AxumHandler<((),), S> for ConduitAxumHandler<H>
where
    S: Send + Sync + 'static,
    H: Handler,
{
    type Future = Pin<Box<dyn Future<Output = AxumResponse> + Send>>;

    fn call(self, request: Request<Body>, _state: S) -> Self::Future {
        Box::pin(async move {
            if let Err(response) = check_content_length(&request) {
                return response.into_response();
            }

            let (parts, body) = request.into_parts();

            let full_body = match hyper::body::to_bytes(body).await {
                Ok(body) => body,
                Err(err) => return server_error_response(&err),
            };
            let request = Request::from_parts(parts, full_body);

            let Self(handler) = self;
            spawn_blocking(move || {
                let mut request = ConduitRequest::new(request);
                handler
                    .call(&mut request)
                    .map(|mut response| {
                        if let Some(pattern) = request.mut_extensions().remove::<RoutePattern>() {
                            response.extensions_mut().insert(pattern);
                        }

                        conduit_into_axum(response)
                    })
                    .unwrap_or_else(|e| server_error_response(&*e))
            })
            .await
            .map_err(ServiceError::from)
            .into_response()
        })
    }
}

#[derive(Clone, Debug)]
pub struct ErrorField(pub String);

#[derive(Clone, Debug)]
pub struct CauseField(pub String);

/// Turns a `ConduitResponse` into a `AxumResponse`
pub fn conduit_into_axum(response: ConduitResponse) -> AxumResponse {
    use conduit::Body::*;

    let (parts, body) = response.into_parts();
    match body {
        Static(slice) => Response::from_parts(parts, axum::body::Body::from(slice)).into_response(),
        Owned(vec) => Response::from_parts(parts, axum::body::Body::from(vec)).into_response(),
        File(file) => Response::from_parts(parts, FileStream::from_std(file).into_streamed_body())
            .into_response(),
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> AxumResponse {
        server_error_response(&self)
    }
}

/// Logs an error message and returns a generic status 500 response
fn server_error_response<E: Error + ?Sized>(error: &E) -> AxumResponse {
    error!(%error, "Internal Server Error");

    sentry_core::capture_error(error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Extension(ErrorField(error.to_string())),
        "Internal Server Error",
    )
        .into_response()
}

/// Check for `Content-Length` values that are invalid or too large
///
/// If a `Content-Length` is provided then `hyper::body::to_bytes()` may try to allocate a buffer
/// of this size upfront, leading to a process abort and denial of service to other clients.
///
/// This only checks for requests that claim to be too large. If the request is chunked then it
/// is possible to allocate larger chunks of memory over time, by actually sending large volumes of
/// data. Request sizes must be limited higher in the stack to protect against this type of attack.
fn check_content_length(request: &Request<Body>) -> Result<(), AxumResponse> {
    fn bad_request(message: &str) -> AxumResponse {
        warn!("Bad request: Content-Length {}", message);

        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("Unexpected invalid header")
            .into_response()
    }

    if let Some(content_length) = request.headers().get(CONTENT_LENGTH) {
        let content_length = match content_length.to_str() {
            Ok(some) => some,
            Err(_) => return Err(bad_request("not ASCII")),
        };

        let content_length = match content_length.parse::<u64>() {
            Ok(some) => some,
            Err(_) => return Err(bad_request("not a u64")),
        };

        if content_length > MAX_CONTENT_LENGTH {
            return Err(bad_request("too large"));
        }
    }

    // A duplicate check, aligning with the specific impl of `hyper::body::to_bytes`
    // (at the time of this writing)
    if request.size_hint().lower() > MAX_CONTENT_LENGTH {
        return Err(bad_request("size_hint().lower() too large"));
    }

    Ok(())
}
