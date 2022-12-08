use crate::adaptor::ConduitRequest;
use crate::file_stream::FileStream;
use crate::service::ServiceError;
use crate::{ConduitResponse, HyperResponse};

use std::net::SocketAddr;
use std::sync::Arc;

use conduit::{Handler, StartInstant};
use http::header::CONTENT_LENGTH;
use http::StatusCode;
use hyper::body::HttpBody;
use hyper::{Body, Request, Response};
use tracing::{error, warn};

/// The maximum size allowed in the `Content-Length` header
///
/// Chunked requests may grow to be larger over time if that much data is actually sent.
/// See the usage section of the README if you plan to use this server in production.
const MAX_CONTENT_LENGTH: u64 = 128 * 1024 * 1024; // 128 MB

#[derive(Debug)]
pub struct BlockingHandler<H: Handler> {
    handler: Arc<H>,
}

impl<H: Handler> BlockingHandler<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    // pub(crate) is for tests
    pub(crate) async fn blocking_handler(
        self: Arc<Self>,
        request: Request<Body>,
        remote_addr: SocketAddr,
    ) -> Result<HyperResponse, ServiceError> {
        if let Err(response) = check_content_length(&request) {
            return Ok(response);
        }

        let (parts, body) = request.into_parts();
        let now = StartInstant::now();

        let full_body = hyper::body::to_bytes(body).await?;
        let request = Request::from_parts(parts, full_body);

        let handler = self.handler.clone();
        tokio::task::spawn_blocking(move || {
            let mut request = ConduitRequest::new(request, remote_addr, now);
            handler
                .call(&mut request)
                .map(conduit_into_hyper)
                .unwrap_or_else(|e| server_error_response(&e.to_string()))
        })
        .await
        .map_err(Into::into)
    }
}

/// Turns a `ConduitResponse` into a `HyperResponse`
fn conduit_into_hyper(response: ConduitResponse) -> HyperResponse {
    use conduit::Body::*;

    let (parts, body) = response.into_parts();
    let body = match body {
        Static(slice) => slice.into(),
        Owned(vec) => vec.into(),
        File(file) => FileStream::from_std(file).into_streamed_body(),
    };
    HyperResponse::from_parts(parts, body)
}

/// Logs an error message and returns a generic status 500 response
fn server_error_response(message: &str) -> HyperResponse {
    error!("Internal Server Error: {}", message);
    let body = hyper::Body::from("Internal Server Error");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(body)
        .expect("Unexpected invalid header")
}

/// Check for `Content-Length` values that are invalid or too large
///
/// If a `Content-Length` is provided then `hyper::body::to_bytes()` may try to allocate a buffer
/// of this size upfront, leading to a process abort and denial of service to other clients.
///
/// This only checks for requests that claim to be too large. If the request is chunked then it
/// is possible to allocate larger chunks of memory over time, by actually sending large volumes of
/// data. Request sizes must be limited higher in the stack to protect against this type of attack.
fn check_content_length(request: &Request<Body>) -> Result<(), HyperResponse> {
    fn bad_request(message: &str) -> HyperResponse {
        warn!("Bad request: Content-Length {}", message);

        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("Unexpected invalid header")
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
