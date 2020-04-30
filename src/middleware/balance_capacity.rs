//! Reject certain requests as instance load reaches capacity.
//!
//! The primary goal of this middleware is to avoid starving the download endpoint of resources.
//! When bots send many parallel requests that run slow database queries, download requests may
//! block and eventually timeout waiting for a database connection.
//!
//! Bots must continue to respect our crawler policy, but until we can manually block bad clients
//! we should avoid dropping download requests even if that means rejecting some legitimate
//! requests to other endpoints.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::prelude::*;
use conduit::{RequestExt, StatusCode};

#[derive(Default)]
pub(super) struct BalanceCapacity {
    handler: Option<Box<dyn Handler>>,
    capacity: usize,
    in_flight_requests: AtomicUsize,
}

impl BalanceCapacity {
    pub fn new(capacity: usize) -> Self {
        Self {
            handler: None,
            capacity,
            in_flight_requests: AtomicUsize::new(0),
        }
    }
}

impl AroundMiddleware for BalanceCapacity {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for BalanceCapacity {
    fn call(&self, request: &mut dyn RequestExt) -> AfterResult {
        // The _drop_on_exit ensures the counter is decremented for all exit paths (including panics)
        let (_drop_on_exit, count) = RequestCounter::add_one(&self.in_flight_requests);
        let handler = self.handler.as_ref().unwrap();
        let load = 100 * count / self.capacity;

        // Begin logging request count at 20% capacity
        if load >= 20 {
            super::log_request::add_custom_metadata(request, "in_flight_requests", count);
        }

        // Download requests are always accepted
        if request.path().starts_with("/api/v1/crates/") && request.path().ends_with("/download") {
            return handler.call(request);
        }

        // Reject read-only requests after reaching 70% load. Bots are likely to send only safe
        // requests and this helps prioritize requests that users may be reluctant to retry.
        if load >= 70 && request.method().is_safe() {
            return over_capcity_response();
        }

        // At 80% load, all non-download requests are rejected
        if load >= 80 {
            return over_capcity_response();
        }

        handler.call(request)
    }
}

fn over_capcity_response() -> AfterResult {
    // TODO: Generate an alert so we can investigate
    let body = "Service temporarily unavailable";
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from_static(body.as_bytes()))
        .map_err(box_error)
}

// FIXME(JTG): I've copied the following from my `conduit-hyper` crate.  Once we transition from
// `civet`, we could pass the in_flight_request count from `condut-hyper` via a request extension.

/// A struct that stores a reference to an atomic counter so it can be decremented when dropped
struct RequestCounter<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> RequestCounter<'a> {
    fn add_one(counter: &'a AtomicUsize) -> (Self, usize) {
        let previous = counter.fetch_add(1, Ordering::SeqCst);
        (Self { counter }, previous + 1)
    }
}

impl<'a> Drop for RequestCounter<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
