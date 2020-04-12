//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::util::request_header;
use conduit::{header, RequestExt, StatusCode};
use std::fmt::{self, Display, Formatter};
use std::time::Instant;

const SLOW_REQUEST_THRESHOLD_MS: u64 = 1000;

#[derive(Default)]
pub(super) struct LogRequests();

struct RequestStart(Instant);

impl Middleware for LogRequests {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        req.mut_extensions().insert(RequestStart(Instant::now()));
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

#[cfg(test)]
pub(crate) fn get_log_message(req: &dyn RequestExt, key: &'static str) -> String {
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
        write!(f, "{}", self.0.path())?;
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
