//! Reject certain requests as instance load reaches capacity.
//!
//! The primary goal of this middleware is to avoid starving the download endpoint of resources.
//! When bots send many parallel requests that run slow database queries, download requests may
//! block and eventually timeout waiting for a database connection.
//!
//! Bots must continue to respect our crawler policy, but until we can manually block bad clients
//! we should avoid dropping download requests even if that means rejecting some legitimate
//! requests to other endpoints.

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::prelude::*;
use conduit::{RequestExt, StatusCode};

#[derive(Default)]
pub(super) struct BalanceCapacity {
    handler: Option<Box<dyn Handler>>,
    capacity: usize,
    in_flight_requests: AtomicUsize,
    report_only: bool,
    log_at_percentage: usize,
    throttle_at_percentage: usize,
    dl_only_at_percentage: usize,
}

impl BalanceCapacity {
    pub fn new(capacity: usize) -> Self {
        Self {
            handler: None,
            capacity,
            in_flight_requests: AtomicUsize::new(0),
            report_only: env::var("WEB_CAPACITY_REPORT_ONLY").ok().is_some(),
            log_at_percentage: read_env_percentage("WEB_CAPACITY_LOG_PCT", 50),
            throttle_at_percentage: read_env_percentage("WEB_CAPACITY_THROTTLE_PCT", 70),
            dl_only_at_percentage: read_env_percentage("WEB_CAPACITY_DL_ONLY_PCT", 80),
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

        // Begin logging request count so early stages of load increase can be located
        if load >= self.log_at_percentage {
            super::log_request::add_custom_metadata(request, "in_flight_requests", count);
        }

        // In report-only mode we serve all requests and only enforce the logging limit above
        if self.report_only {
            return handler.call(request);
        }

        // Download requests are always accepted
        if request.path().starts_with("/api/v1/crates/") && request.path().ends_with("/download") {
            return handler.call(request);
        }

        // Reject read-only requests as load nears capacity. Bots are likely to send only safe
        // requests and this helps prioritize requests that users may be reluctant to retry.
        if load >= self.throttle_at_percentage && request.method().is_safe() {
            return over_capacity_response(request);
        }

        // As load reaches capacity, all non-download requests are rejected
        if load >= self.dl_only_at_percentage {
            return over_capacity_response(request);
        }

        handler.call(request)
    }
}

fn over_capacity_response(request: &mut dyn RequestExt) -> AfterResult {
    // TODO: Generate an alert so we can investigate
    super::log_request::add_custom_metadata(request, "cause", "over capacity");
    let body = "Service temporarily unavailable";
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from_static(body.as_bytes()))
        .map_err(box_error)
}

fn read_env_percentage(name: &str, default: usize) -> usize {
    if let Ok(value) = env::var(name) {
        value.parse().unwrap_or(default)
    } else {
        default
    }
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
