use crate::app::AppState;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;
use conduit_router::RoutePattern;
use http::Request;
use std::time::Instant;

pub async fn update_metrics<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let start_instant = Instant::now();

    let metrics = &state.instance_metrics;
    metrics.requests_in_flight.inc();

    let response = next.run(req).await;

    metrics.requests_in_flight.dec();
    metrics.requests_total.inc();

    let endpoint = response
        .extensions()
        .get::<RoutePattern>()
        .map(|p| p.pattern())
        .unwrap_or("<unknown>");
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
