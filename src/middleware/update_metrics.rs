use super::app::RequestApp;
use super::prelude::*;

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

        res
    }
}
