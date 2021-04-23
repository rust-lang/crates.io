use crate::controllers::frontend_prelude::*;
use crate::util::errors::{forbidden, not_found, MetricsDisabled};
use conduit::{Body, Response};
use prometheus::{Encoder, TextEncoder};

/// Handles the `GET /api/private/metrics/:kind` endpoint.
pub fn prometheus(req: &mut dyn RequestExt) -> EndpointResult {
    let app = req.app();

    if let Some(expected_token) = &app.config.metrics_authorization_token {
        let provided_token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        if provided_token != Some(expected_token.as_str()) {
            return Err(forbidden());
        }
    } else {
        // To avoid accidentally leaking metrics if the environment variable is not set, prevent
        // access to any metrics endpoint if the authorization token is not configured.
        return Err(Box::new(MetricsDisabled));
    }

    let metrics = match req.params()["kind"].as_str() {
        "service" => app.service_metrics.gather(&*req.db_read_only()?)?,
        "instance" => app.instance_metrics.gather(app)?,
        _ => return Err(not_found()),
    };

    let mut output = Vec::new();
    TextEncoder::new().encode(&metrics, &mut output)?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CONTENT_LENGTH, output.len())
        .body(Body::from_vec(output))?)
}
