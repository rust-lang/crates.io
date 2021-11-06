use crate::middleware::response_timing::ResponseTime;
use conduit::{RequestExt, StatusCode};
use conduit_cookie::RequestSession;
use conduit_middleware::{AfterResult, BeforeResult, Middleware};
use sentry_conduit::SentryMiddleware;

/// Custom wrapper around the `sentry_conduit` middleware, that adds additional
/// metadata to the Sentry request scopes.
#[derive(Default)]
pub struct CustomSentryMiddleware {
    inner: SentryMiddleware,
}

impl Middleware for CustomSentryMiddleware {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        self.inner.before(req)?;

        if let Some(request_id) = req
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
        {
            sentry::configure_scope(|scope| scope.set_tag("request.id", request_id));
        }

        Ok(())
    }

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

            {
                let status = res
                    .as_ref()
                    .map(|resp| resp.status())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                scope.set_tag("response.status", status.as_str());
            }

            if let Some(response_time) = req.extensions().get::<ResponseTime>() {
                scope.set_extra("Response time [ms]", response_time.as_millis().into());
            }
        });

        self.inner.after(req, res)
    }
}
