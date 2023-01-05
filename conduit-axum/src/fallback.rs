use crate::error::ServiceError;
use crate::response::AxumResponse;
use crate::{spawn_blocking, ConduitRequest, Handler};

use std::collections::BTreeMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::{Body, HttpBody};
use axum::extract::{Extension, FromRequest, Path};
use axum::handler::Handler as AxumHandler;
use axum::response::IntoResponse;
use http::header::CONTENT_LENGTH;
use http::StatusCode;
use hyper::Request;
use tracing::{error, warn};

/// The maximum size allowed in the `Content-Length` header
///
/// Chunked requests may grow to be larger over time if that much data is actually sent.
/// See the usage section of the README if you plan to use this server in production.
const MAX_CONTENT_LENGTH: u64 = 128 * 1024 * 1024; // 128 MB

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

    fn call(self, request: Request<Body>, state: S) -> Self::Future {
        Box::pin(async move {
            let request = match ConduitRequest::from_request(request, &state).await {
                Ok(request) => request,
                Err(err) => return err,
            };

            let Self(handler) = self;
            spawn_blocking(move || handler.call(request))
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

impl IntoResponse for ServiceError {
    fn into_response(self) -> AxumResponse {
        server_error_response(&self)
    }
}

/// Logs an error message and returns a generic status 500 response
pub fn server_error_response<E: Error + ?Sized>(error: &E) -> AxumResponse {
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
pub(crate) fn check_content_length(request: &Request<Body>) -> Result<(), AxumResponse> {
    fn bad_request(message: &str) -> AxumResponse {
        warn!("Bad request: Content-Length {}", message);
        StatusCode::BAD_REQUEST.into_response()
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

pub type Params = Path<BTreeMap<String, String>>;

pub trait RequestParamsExt<'a> {
    fn axum_params(self) -> Option<&'a Params>;
}

impl<'a> RequestParamsExt<'a> for &'a ConduitRequest {
    fn axum_params(self) -> Option<&'a Params> {
        self.extensions().get::<Params>()
    }
}
