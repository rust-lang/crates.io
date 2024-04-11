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

use crate::app::AppState;
use crate::middleware::log_request::RequestLogExt;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

/// Handle a request when load exceeds a threshold
///
/// In report-only mode, log metadata is added but the request is still served. Otherwise,
/// the request is rejected with a service unavailable response.
async fn handle_high_load(
    app_state: &AppState,
    request: Request,
    next: Next,
    note: &str,
) -> Response {
    let config = &app_state.config.balance_capacity;
    if config.report_only {
        // In report-only mode we serve all requests but add log metadata
        request.request_log().add("would_reject", note);
        next.run(request).await
    } else {
        // Reject the request
        request.request_log().add("cause", note);

        let body = "Service temporarily unavailable";
        (StatusCode::SERVICE_UNAVAILABLE, body).into_response()
    }
}

pub async fn balance_capacity(app_state: AppState, request: Request, next: Next) -> Response {
    let config = &app_state.config.balance_capacity;
    let db_capacity = app_state.config.db.primary.pool_size;
    let state = &app_state.balance_capacity;

    let request_log = request.request_log();

    // The _drop_on_exit ensures the counter is decremented for all exit paths (including panics)
    let (_drop_on_exit1, in_flight_total) = RequestCounter::add_one(&state.in_flight_total);

    // Begin logging total request count so early stages of load increase can be located
    if in_flight_total >= config.log_total_at_count {
        request_log.add("in_flight_total", in_flight_total);
    }

    // Download requests are always accepted and do not affect the capacity tracking
    let path = request.uri().path();
    if path.starts_with("/api/v1/crates/") && path.ends_with("/download") {
        return next.run(request).await;
    }

    // The _drop_on_exit ensures the counter is decremented for all exit paths (including panics)
    let (_drop_on_exit2, count) = RequestCounter::add_one(&state.in_flight_non_dl_requests);
    let load = 100 * count / db_capacity;

    // Begin logging non-download request count so early stages of non-download load increase can be located
    if load >= config.log_at_percentage {
        request_log.add("in_flight_non_dl_requests", count);
    }

    // Reject read-only requests as load nears capacity. Bots are likely to send only safe
    // requests and this helps prioritize requests that users may be reluctant to retry.
    if load >= config.throttle_at_percentage && request.method().is_safe() {
        return handle_high_load(&app_state, request, next, "over capacity (throttle)").await;
    }

    // As load reaches capacity, all non-download requests are rejected
    if load >= config.dl_only_at_percentage {
        return handle_high_load(&app_state, request, next, "over capacity (download only)").await;
    }

    next.run(request).await
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
