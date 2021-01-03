use crate::adaptor::{ConduitRequest, RequestInfo};
use crate::file_stream::FileStream;
use crate::service::ServiceError;
use crate::{ConduitResponse, HyperResponse};

use std::net::SocketAddr;
use std::sync::Arc;

use conduit::{Handler, StartInstant, StatusCode};
use hyper::{Body, Request, Response};
use tracing::error;

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
        let (parts, body) = request.into_parts();
        let now = StartInstant::now();

        let full_body = hyper::body::to_bytes(body).await?;
        let mut request_info = RequestInfo::new(parts, full_body);

        let handler = self.handler.clone();
        tokio::task::spawn_blocking(move || {
            let mut request = ConduitRequest::new(&mut request_info, remote_addr, now);
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
