use crate::config::SentryConfig;
use http::header::AUTHORIZATION;
use sentry::protocol::Event;
use sentry::{ClientInitGuard, ClientOptions, TransactionContext};
use std::sync::Arc;

/// Initializes the Sentry SDK from the environment variables.
///
/// If `SENTRY_DSN_API` is not set then Sentry will not be initialized,
/// otherwise it is required to be a valid DSN string. `SENTRY_ENV_API` must
/// be set if a DSN is provided.
///
/// `HEROKU_SLUG_COMMIT`, if present, will be used as the `release` property
/// on all events.
pub fn init() -> Option<ClientInitGuard> {
    let config = match SentryConfig::from_environment() {
        Ok(config) => config,
        Err(error) => {
            warn!(%error, "Failed to read Sentry configuration from environment");
            return None;
        }
    };

    Some(sentry::init(options(config)))
}

fn options(config: SentryConfig) -> ClientOptions {
    let traces_sampler = move |ctx: &TransactionContext| -> f32 {
        if let Some(sampled) = ctx.sampled() {
            return if sampled { 1.0 } else { 0.0 };
        }

        let op = ctx.operation();
        if op == "http.server" {
            let is_download_endpoint =
                ctx.name().starts_with("GET /api/v1/crates/") && ctx.name().ends_with("/download");

            if is_download_endpoint {
                // Reduce the sample rate for the download endpoint, since we have significantly
                // more traffic on that endpoint compared to the rest
                return config.traces_sample_rate / 100.;
            } else if ctx.name() == "PUT /api/v1/crates/new" {
                // Record all traces for crate publishing
                return 1.;
            } else if ctx.name().starts_with("GET /api/private/metrics/") {
                // Ignore all traces for internal metrics collection
                return 0.;
            }
        } else if op == "swirl.perform" || op == "admin.command" {
            // Record all traces for background tasks and admin commands
            return 1.;
        } else if op == "swirl.run" || op == "server.run" {
            // Ignore top-level span from the background worker and http server
            return 0.;
        }

        config.traces_sample_rate
    };

    let before_send = |mut event: Event<'static>| {
        if let Some(request) = &mut event.request {
            // Remove cookies from the request to avoid sending sensitive information like the
            // `cargo_session`.
            request.cookies.take();

            // Also remove `Authorization`, just so it never even gets sent to Sentry, even if
            // they're redacting it downstream.
            request
                .headers
                .retain(|name, _value| AUTHORIZATION != name.as_str());
        }

        Some(event)
    };

    ClientOptions {
        auto_session_tracking: true,
        dsn: config.dsn,
        environment: config.environment.map(Into::into),
        release: config.release.map(Into::into),
        before_send: Some(Arc::new(before_send)),
        session_mode: sentry::SessionMode::Request,
        traces_sampler: Some(Arc::new(traces_sampler)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sentry::{
        capture_error,
        protocol::{Request, SpanStatus, Url},
        start_transaction,
    };

    #[test]
    fn test_redaction() -> anyhow::Result<()> {
        let req = Request {
            url: Some(Url::parse("https://crates.io/api/v1/foo")?),
            method: Some("GET".into()),
            data: None,
            cookies: Some("cargo_session=foobar".into()),
            headers: [
                ("Authorization", "secret"),
                ("authorization", "another secret"),
                ("Accept", "application/json"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
            query_string: None,
            env: Default::default(),
        };
        let err = std::io::Error::other("error");

        let opts = options(SentryConfig::default());
        let event_req = req.clone();
        let mut events = sentry::test::with_captured_events_options(
            move || {
                let scope_req = event_req.clone();
                sentry::configure_scope(|scope| {
                    // This is straight up replicated from the implementation of SentryHttpFuture,
                    // and is how requests are attached by the Tower middleware.
                    scope.add_event_processor(move |mut event| {
                        if event.request.is_none() {
                            event.request = Some(scope_req.clone());
                        }
                        Some(event)
                    });
                });

                let ctx = TransactionContext::new("test", "http.server");
                let txn = start_transaction(ctx);
                txn.set_request(event_req);
                txn.set_status(SpanStatus::InternalError);

                capture_error(&err);

                txn.finish();
            },
            opts,
        );

        // OK, so there should be exactly one event, and it should match `req` except that its
        // cookies are removed and its headers have been cleaned of all Authorization values. Let's
        // see what we actually have.
        assert_eq!(events.len(), 1);
        let event = assert_some!(events.pop());
        let event_req = assert_some!(event.request);

        // Things that shouldn't change.
        assert_eq!(&req.url, &event_req.url);
        assert_eq!(&req.method, &event_req.method);
        assert_eq!(&req.data, &event_req.data);
        assert_eq!(&req.query_string, &event_req.query_string);
        assert_eq!(&req.env, &event_req.env);

        // Things that should.
        assert_none!(&event_req.cookies);
        assert_eq!(event_req.headers.len(), 1);
        assert_some!(event_req.headers.get("Accept"));

        Ok(())
    }
}
