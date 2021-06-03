use super::app::RequestApp;
use super::prelude::*;
use conduit_router::RoutePattern;

#[derive(Debug, Default)]
pub(super) struct UpdateMetrics;

impl Middleware for UpdateMetrics {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        let metrics = &req.app().instance_metrics;

        metrics.requests_in_flight.inc();

        Ok(())
    }

    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        let metrics = &req.app().instance_metrics;

        metrics.requests_in_flight.dec();
        metrics.requests_total.inc();

        let endpoint = req
            .extensions()
            .find::<RoutePattern>()
            .map(|p| p.pattern())
            .unwrap_or("<unknown>");
        metrics
            .response_times
            .with_label_values(&[endpoint])
            .observe(req.elapsed().as_millis() as f64 / 1000.0);

        res
    }
}
