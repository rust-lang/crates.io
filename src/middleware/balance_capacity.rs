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
    in_flight_total: AtomicUsize,
    db_capacity: usize,
    in_flight_non_dl_requests: AtomicUsize,
    report_only: bool,
    log_total_at_count: usize,
    log_at_percentage: usize,
    throttle_at_percentage: usize,
    dl_only_at_percentage: usize,
}

impl BalanceCapacity {
    pub fn new(db_capacity: usize) -> Self {
        Self {
            handler: None,
            in_flight_total: AtomicUsize::new(0),
            db_capacity,
            in_flight_non_dl_requests: AtomicUsize::new(0),

            report_only: env::var("WEB_CAPACITY_REPORT_ONLY").ok().is_some(),
            log_total_at_count: read_env_percentage("WEB_CAPACITY_LOG_TOTAL_AT_COUNT", 50),
            // The following are a percentage of `db_capacity`
            log_at_percentage: read_env_percentage("WEB_CAPACITY_LOG_PCT", 50),
            throttle_at_percentage: read_env_percentage("WEB_CAPACITY_THROTTLE_PCT", 70),
            dl_only_at_percentage: read_env_percentage("WEB_CAPACITY_DL_ONLY_PCT", 80),
        }
    }

    /// Handle a request normally
    fn handle(&self, request: &mut dyn RequestExt) -> AfterResult {
        self.handler.as_ref().unwrap().call(request)
    }

    /// Handle a request when load exceeds a threshold
    ///
    /// In report-only mode, log metadata is added but the request is still served. Otherwise,
    /// the request is rejected with a service unavailable response.
    fn handle_high_load(&self, request: &mut dyn RequestExt, note: &str) -> AfterResult {
        if self.report_only {
            // In report-only mode we serve all requests but add log metadata
            add_custom_metadata(request, "would_reject", note);
            self.handle(request)
        } else {
            // Reject the request
            add_custom_metadata(request, "cause", note);
            let body = "Service temporarily unavailable";
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header(header::CONTENT_LENGTH, body.len())
                .body(Body::from_static(body.as_bytes()))
                .map_err(box_error)
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
        let (_drop_on_exit1, in_flight_total) = RequestCounter::add_one(&self.in_flight_total);

        // Begin logging total request count so early stages of load increase can be located
        if in_flight_total >= self.log_total_at_count {
            add_custom_metadata(request, "in_flight_total", in_flight_total);
        }

        // Download requests are always accepted and do not affect the capacity tracking
        if request.path().starts_with("/api/v1/crates/") && request.path().ends_with("/download") {
            return self.handle(request);
        }

        // The _drop_on_exit ensures the counter is decremented for all exit paths (including panics)
        let (_drop_on_exit2, count) = RequestCounter::add_one(&self.in_flight_non_dl_requests);
        let load = 100 * count / self.db_capacity;

        // Begin logging non-download request count so early stages of non-download load increase can be located
        if load >= self.log_at_percentage {
            add_custom_metadata(request, "in_flight_non_dl_requests", count);
        }

        // Reject read-only requests as load nears capacity. Bots are likely to send only safe
        // requests and this helps prioritize requests that users may be reluctant to retry.
        if load >= self.throttle_at_percentage && request.method().is_safe() {
            return self.handle_high_load(request, "over capacity (throttle)");
        }

        // As load reaches capacity, all non-download requests are rejected
        if load >= self.dl_only_at_percentage {
            return self.handle_high_load(request, "over capacity (download only)");
        }

        self.handle(request)
    }
}

fn read_env_percentage(name: &str, default: usize) -> usize {
    if let Ok(value) = env::var(name) {
        value.parse().unwrap_or(default)
    } else {
        default
    }
}

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
