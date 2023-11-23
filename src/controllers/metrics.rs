use crate::controllers::frontend_prelude::*;
use crate::util::errors::{forbidden, not_found, MetricsDisabled};
use axum::response::IntoResponse;
use prometheus::{Encoder, TextEncoder};

/// Handles the `GET /api/private/metrics/:kind` endpoint.
pub async fn prometheus(
    app: AppState,
    Path(kind): Path<String>,
    req: Parts,
) -> AppResult<Response> {
    spawn_blocking(move || {
        if let Some(expected_token) = &app.config.metrics_authorization_token {
            let provided_token = req
                .headers
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

        let metrics = match kind.as_str() {
            "service" => app.service_metrics.gather(&mut *app.db_read()?)?,
            "instance" => app.instance_metrics.gather(&app)?,
            _ => return Err(not_found()),
        };

        let mut output = Vec::new();
        TextEncoder::new().encode(&metrics, &mut output)?;

        Ok((
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            output,
        )
            .into_response())
    })
    .await
}
