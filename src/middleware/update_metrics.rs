use crate::app::AppState;
use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

use prometheus::IntGauge;
use std::time::Instant;

pub async fn update_metrics(
    state: AppState,
    matched_path: Option<MatchedPath>,
    req: Request,
    next: Next,
) -> Response {
    let start_instant = Instant::now();

    let metrics = &state.instance_metrics;
    let _guard = GaugeGuard::inc_for(&metrics.requests_in_flight);

    let response = next.run(req).await;

    metrics.requests_total.inc();

    let endpoint = match matched_path {
        Some(ref matched_path) => matched_path.as_str(),
        None => "<unknown>",
    };
    metrics
        .response_times
        .with_label_values(&[endpoint])
        .observe(start_instant.elapsed().as_millis() as f64 / 1000.0);

    let status = response.status().as_u16();
    metrics
        .responses_by_status_code_total
        .with_label_values(&[&status.to_string()])
        .inc();

    response
}

/// A struct that stores a reference to an `IntGauge` so it can be decremented when dropped
struct GaugeGuard<'a> {
    gauge: &'a IntGauge,
}

impl<'a> GaugeGuard<'a> {
    fn inc_for(gauge: &'a IntGauge) -> Self {
        gauge.inc();
        Self { gauge }
    }
}

impl Drop for GaugeGuard<'_> {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}
