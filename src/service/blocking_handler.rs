use crate::adaptor::{ConduitRequest, RequestInfo};
use crate::file_stream::FileStream;
use crate::service::ServiceError;
use crate::{ConduitResponse, HyperResponse};

use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use conduit::{header, Handler, StatusCode};
use hyper::{Body, Request, Response};
use tracing::error;

#[derive(Debug)]
pub struct BlockingHandler<H: Handler> {
    thread_count: AtomicUsize,
    max_thread_count: usize,
    handler: Arc<H>,
}

impl<H: Handler> BlockingHandler<H> {
    pub fn new(handler: H, max_thread_count: usize) -> Self {
        Self {
            thread_count: AtomicUsize::new(0),
            max_thread_count,
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

        let full_body = hyper::body::to_bytes(body).await?;
        let mut request_info = RequestInfo::new(parts, full_body);

        // The _drop_on_return ensures the counter is decreased for all exit paths
        let (_drop_on_return, prev_count) = ThreadCounter::begin_with(&self.thread_count);

        // Comparison is against the "previous value" from an atomic fetch_add, so using `>=`
        if prev_count >= self.max_thread_count {
            return Ok(over_capacity_error_response());
        }

        let handler = self.handler.clone();
        tokio::task::spawn_blocking(move || {
            let mut request = ConduitRequest::new(&mut request_info, remote_addr);
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

/// Logs an error message and returns a 503 status saying the service is over capacity
fn over_capacity_error_response() -> HyperResponse {
    const RETRY_AFTER: u32 = 2;
    error!("Server Capacity Exceeded");
    let body = hyper::Body::from(format!(
        "Service Unavailable: Please retry after {} seconds.",
        RETRY_AFTER
    ));
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::RETRY_AFTER, RETRY_AFTER)
        .body(body)
        .expect("Unexpected invalid header")
}

/// A struct that stores a reference to an atomic counter so it can be decremented when dropped
struct ThreadCounter<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> ThreadCounter<'a> {
    fn begin_with(counter: &'a AtomicUsize) -> (Self, usize) {
        let previous = counter.fetch_add(1, Ordering::SeqCst);
        (Self { counter }, previous)
    }
}

impl<'a> Drop for ThreadCounter<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
