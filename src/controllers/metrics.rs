use crate::controllers::frontend_prelude::*;
use crate::util::errors::{custom, forbidden, not_found};
use prometheus::TextEncoder;

/// Handles the `GET /api/private/metrics/:kind` endpoint.
pub async fn prometheus(app: AppState, Path(kind): Path<String>, req: Parts) -> AppResult<String> {
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
        let detail = "Metrics are disabled on this crates.io instance";
        return Err(custom(StatusCode::NOT_FOUND, detail));
    }

    let metrics = match kind.as_str() {
        "service" => {
            spawn_blocking(move || app.service_metrics.gather(&mut *app.db_read()?)).await?
        }
        "instance" => {
            spawn_blocking(move || Ok::<_, BoxedAppError>(app.instance_metrics.gather(&app)?))
                .await?
        }
        _ => return Err(not_found()),
    };

    Ok(TextEncoder::new().encode_to_string(&metrics)?)
}
