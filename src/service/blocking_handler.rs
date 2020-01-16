use crate::adaptor::{ConduitRequest, RequestInfo};
use crate::service::ServiceError;

use std::net::SocketAddr;
use std::sync::{atomic::Ordering, Arc};

use hyper::{Body, Request, Response, StatusCode};
use tracing::error;

use std::sync::atomic::AtomicUsize;

#[derive(Debug)]
pub struct BlockingHandler<H: conduit::Handler> {
    thread_count: AtomicUsize,
    max_thread_count: usize,
    handler: Arc<H>,
}

impl<H: conduit::Handler> BlockingHandler<H> {
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
    ) -> Result<Response<Body>, ServiceError> {
        let (parts, body) = request.into_parts();

        let full_body = hyper::body::to_bytes(body).await?;
        let mut request_info = RequestInfo::new(parts, full_body);

        // The _drop_handler ensures the counter is decreased for all exit paths
        let (_drop_handler, count) = ThreadCounter::new_and_add(&self.thread_count);

        // Use `>=` for comparison because count is the "previous value" of the counter
        if count >= self.max_thread_count {
            return Ok(over_capacity_error_response());
        }

        let handler = self.handler.clone();
        tokio::task::spawn_blocking(move || {
            let mut request = ConduitRequest::new(&mut request_info, remote_addr);
            handler
                .call(&mut request)
                .map(good_response)
                .unwrap_or_else(|e| server_error_response(&e.to_string()))
        })
        .await
        .map_err(Into::into)
    }
}

/// Builds a `hyper::Response` given a `conduit:Response`
fn good_response(mut response: conduit::Response) -> Response<Body> {
    let mut body = Vec::new();
    if response.body.write_body(&mut body).is_err() {
        return server_error_response("Error writing body");
    }

    let mut builder = Response::builder();
    let status = match StatusCode::from_u16(response.status.0 as u16) {
        Ok(s) => s,
        Err(e) => return server_error_response(&e.to_string()),
    };

    for (key, values) in response.headers {
        for value in values {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }

    builder
        .status(status)
        .body(body.into())
        .unwrap_or_else(|e| server_error_response(&e.to_string()))
}

/// Logs an error message and returns a generic status 500 response
fn server_error_response(message: &str) -> Response<Body> {
    error!("Internal Server Error: {}", message);
    let body = Body::from("Internal Server Error");
    Response::builder()
        .status(500)
        .body(body)
        .expect("Unexpected invalid header")
}

/// Logs an error message and returns a 503 status saying the service is over capacity
fn over_capacity_error_response() -> Response<Body> {
    const RETRY_AFTER: u32 = 2;
    error!("Server Capacity Exceeded");
    let body = Body::from(format!(
        "Service Unavailable: Please retry after {} seconds.",
        RETRY_AFTER
    ));
    Response::builder()
        .status(503)
        .header("Retry-After", RETRY_AFTER)
        .body(body)
        .expect("Unexpected invalid header")
}

/// A struct that stores a reference to an atomic counter so it can be decremented when dropped
struct ThreadCounter<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> ThreadCounter<'a> {
    fn new_and_add(counter: &'a AtomicUsize) -> (Self, usize) {
        let count = counter.fetch_add(1, Ordering::SeqCst);
        (Self { counter }, count)
    }
}

impl<'a> Drop for ThreadCounter<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
