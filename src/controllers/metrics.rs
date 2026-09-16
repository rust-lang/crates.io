use crate::metrics::ServiceMetricsSnapshot;
use crate::server::ServerContext;
use crate::tasks::spawn_blocking;
use crate::util::errors::{AppResult, custom, forbidden, not_found};
use crate::util::no_store;
use axum::extract::Path;
use axum_extra::TypedHeader;
use axum_extra::headers::CacheControl;
use http::request::Parts;
use http::{StatusCode, header};
use prometheus::TextEncoder;
use secrecy::ExposeSecret;

/// Handles the `GET /api/private/metrics/{kind}` endpoint.
pub async fn prometheus(
    ctx: ServerContext,
    Path(kind): Path<String>,
    req: Parts,
) -> AppResult<(TypedHeader<CacheControl>, String)> {
    if let Some(expected_token) = &ctx.config.metrics.authorization_token {
        let provided_token = req
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        if provided_token != Some(expected_token.expose_secret()) {
            return Err(forbidden("invalid or missing authorization token"));
        }
    } else {
        // To avoid accidentally leaking metrics if the environment variable is not set, prevent
        // access to any metrics endpoint if the authorization token is not configured.
        let detail = "Metrics are disabled on this crates.io instance";
        return Err(custom(StatusCode::NOT_FOUND, detail));
    }

    let metrics = match kind.as_str() {
        "service" => {
            let mut conn = ctx.db_read().await?;
            let snapshot = ServiceMetricsSnapshot::load(&mut conn).await?;
            ctx.service_metrics.record(snapshot)?
        }
        "instance" => spawn_blocking(move || ctx.instance_metrics.gather(&ctx)).await??,
        _ => return Err(not_found()),
    };

    Ok((no_store(), TextEncoder::new().encode_to_string(&metrics)?))
}
