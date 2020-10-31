//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::middleware::current_user::TrustedUserId;
use crate::util::request_header;
use conduit::{header, Host, RequestExt, Scheme, StatusCode};
use sentry::Level;
use std::fmt::{self, Display, Formatter};
use std::time::Instant;

const SLOW_REQUEST_THRESHOLD_MS: u64 = 1000;

const FILTERED_HEADERS: &[&str] = &["Authorization", "Cookie", "X-Real-Ip", "X-Forwarded-For"];

#[derive(Default)]
pub(super) struct LogRequests();

struct RequestStart(Instant);
struct OriginalPath(String);

impl Middleware for LogRequests {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        req.mut_extensions().insert(RequestStart(Instant::now()));
        let path = OriginalPath(req.path().to_string());
        req.mut_extensions().insert(path);
        Ok(())
    }

    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        // Unwrap shouldn't panic as no other code has access to the private struct to remove it
        let request_start = req.extensions().find::<RequestStart>().unwrap().0;

        let response_time = request_start.elapsed();
        let response_time =
            response_time.as_secs() * 1000 + u64::from(response_time.subsec_nanos()) / 1_000_000;

        println!(
            "{}",
            RequestLine {
                req,
                res: &res,
                response_time,
            }
        );

        report_to_sentry(req, &res, response_time);

        res
    }
}

struct CustomMetadata {
    entries: Vec<(&'static str, String)>,
}

pub fn add_custom_metadata<V: Display>(req: &mut dyn RequestExt, key: &'static str, value: V) {
    if let Some(metadata) = req.mut_extensions().find_mut::<CustomMetadata>() {
        metadata.entries.push((key, value.to_string()));
    } else {
        let mut metadata = CustomMetadata {
            entries: Vec::new(),
        };
        metadata.entries.push((key, value.to_string()));
        req.mut_extensions().insert(metadata);
    }
}

fn report_to_sentry(req: &dyn RequestExt, res: &AfterResult, response_time: u64) {
    let (message, level) = match res {
        Err(e) => (e.to_string(), Level::Error),
        Ok(_) => {
            if response_time <= SLOW_REQUEST_THRESHOLD_MS {
                return;
            }

            let message = format!("Slow Request: {} {}", req.method(), req.path());
            (message, Level::Info)
        }
    };

    let config = |scope: &mut sentry::Scope| {
        let method = Some(req.method().as_str().to_owned());

        let scheme = match req.scheme() {
            Scheme::Http => "http",
            Scheme::Https => "https",
        };

        let host = match req.host() {
            Host::Name(name) => name.to_owned(),
            Host::Socket(addr) => addr.to_string(),
        };

        let path = &req.extensions().find::<OriginalPath>().unwrap().0;

        let url = format!("{}://{}{}", scheme, host, path).parse().ok();

        {
            let id = req
                .extensions()
                .find::<TrustedUserId>()
                .map(|x| x.0.to_string());

            let user = sentry::User {
                id,
                ..Default::default()
            };

            scope.set_user(Some(user));
        }

        {
            let headers = req
                .headers()
                .iter()
                .filter(|(k, _v)| !FILTERED_HEADERS.iter().any(|name| k == name))
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect();

            let sentry_req = sentry::protocol::Request {
                method,
                url,
                headers,
                ..Default::default()
            };

            scope.add_event_processor(Box::new(move |mut event| {
                if event.request.is_none() {
                    event.request = Some(sentry_req.clone());
                }
                Some(event)
            }));
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

        scope.set_extra("Response time [ms]", response_time.into());

        if let Some(metadata) = req.extensions().find::<CustomMetadata>() {
            for (key, value) in &metadata.entries {
                scope.set_extra(key, value.to_string().into());
            }
        }
    };

    sentry::with_scope(config, || sentry::capture_message(&message, level));
}

#[cfg(test)]
pub(crate) fn get_log_message(req: &dyn RequestExt, key: &'static str) -> String {
    // Unwrap shouldn't panic as no other code has access to the private struct to remove it
    for (k, v) in &req.extensions().find::<CustomMetadata>().unwrap().entries {
        if key == *k {
            return v.clone();
        }
    }
    panic!("expected log message for {} not found", key);
}

struct RequestLine<'r> {
    req: &'r dyn RequestExt,
    res: &'r AfterResult,
    response_time: u64,
}

impl Display for RequestLine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        let (at, status) = match self.res {
            Ok(resp) => ("info", resp.status()),
            Err(_) => ("error", StatusCode::INTERNAL_SERVER_ERROR),
        };

        line.add_field("at", at)?;
        line.add_field("method", self.req.method())?;
        line.add_quoted_field("path", FullPath(self.req))?;
        line.add_field("request_id", request_header(self.req, "x-request-id"))?;
        line.add_quoted_field("fwd", request_header(self.req, "x-real-ip"))?;
        line.add_field("service", TimeMs(self.response_time))?;
        line.add_field("status", status.as_str())?;
        line.add_quoted_field("user_agent", request_header(self.req, header::USER_AGENT))?;

        if let Some(metadata) = self.req.extensions().find::<CustomMetadata>() {
            for (key, value) in &metadata.entries {
                line.add_quoted_field(key, value)?;
            }
        }

        if let Err(err) = self.res {
            line.add_quoted_field("error", err)?;
        }

        if self.response_time > SLOW_REQUEST_THRESHOLD_MS {
            line.add_marker("SLOW REQUEST")?;
        }

        Ok(())
    }
}

struct FullPath<'a>(&'a dyn RequestExt);

impl<'a> Display for FullPath<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Unwrap shouldn't panic as no other code has access to the private struct to remove it
        write!(
            f,
            "{}",
            self.0.extensions().find::<OriginalPath>().unwrap().0
        )?;
        if let Some(q_string) = self.0.query_string() {
            write!(f, "?{}", q_string)?;
        }
        Ok(())
    }
}

struct TimeMs(u64);

impl Display for TimeMs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)?;
        f.write_str("ms")?;
        Ok(())
    }
}

struct LogLine<'f, 'g> {
    f: &'f mut Formatter<'g>,
    first: bool,
}

impl<'f, 'g> LogLine<'f, 'g> {
    fn new(f: &'f mut Formatter<'g>) -> Self {
        Self { f, first: true }
    }

    fn add_field<K: Display, V: Display>(&mut self, key: K, value: V) -> fmt::Result {
        self.start_item()?;

        key.fmt(self.f)?;
        self.f.write_str("=")?;
        value.fmt(self.f)?;

        Ok(())
    }

    fn add_quoted_field<K: Display, V: Display>(&mut self, key: K, value: V) -> fmt::Result {
        self.start_item()?;

        key.fmt(self.f)?;
        self.f.write_str("=\"")?;
        value.fmt(self.f)?;
        self.f.write_str("\"")?;

        Ok(())
    }

    fn add_marker<M: Display>(&mut self, marker: M) -> fmt::Result {
        self.start_item()?;

        marker.fmt(self.f)?;

        Ok(())
    }

    fn start_item(&mut self) -> fmt::Result {
        if !self.first {
            self.f.write_str(" ")?;
        }
        self.first = false;
        Ok(())
    }
}
