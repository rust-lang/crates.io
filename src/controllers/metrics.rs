use crate::controllers::frontend_prelude::*;
use crate::util::errors::not_found;
use conduit::{Body, Response};
use prometheus::{Encoder, TextEncoder};

/// Handles the `GET /api/private/metrics/:kind` endpoint.
pub fn prometheus(req: &mut dyn RequestExt) -> EndpointResult {
    let app = req.app();

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
