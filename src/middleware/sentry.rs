use super::prelude::*;
use crate::middleware::log_request::CustomMetadata;
use crate::middleware::request_timing::ResponseTime;
use conduit::{RequestExt, StatusCode};
use conduit_cookie::RequestSession;

#[derive(Default)]
pub struct SentryMiddleware();

impl Middleware for SentryMiddleware {
    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        sentry::configure_scope(|scope| {
            {
                let id = req.session().get("user_id").map(|str| str.to_string());

                let user = sentry::User {
                    id,
                    ..Default::default()
                };

                scope.set_user(Some(user));
            }

            if let Some(request_id) = req
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
            {
                scope.set_tag("request.id", request_id);
            }

            {
                let status = res
                    .as_ref()
                    .map(|resp| resp.status())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                scope.set_tag("response.status", status.as_str());
            }

            let response_time = req.extensions().find::<ResponseTime>();
            if let Some(response_time) = response_time {
                scope.set_extra("Response time [ms]", response_time.as_millis().into());
            }

            if let Some(metadata) = req.extensions().find::<CustomMetadata>() {
                for (key, value) in &metadata.entries {
                    scope.set_extra(key, value.to_string().into());
                }
            }
        });

        res
    }
}
