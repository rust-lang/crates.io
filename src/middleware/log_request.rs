//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::util::request_header;
use conduit::Request;
use std::fmt::{self, Display, Formatter};
use std::time::Instant;

const SLOW_REQUEST_THRESHOLD_MS: u64 = 1000;

#[allow(missing_debug_implementations)] // We can't
#[derive(Default)]
pub struct LogRequests {
    handler: Option<Box<dyn Handler>>,
}

impl AroundMiddleware for LogRequests {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for LogRequests {
    fn call(&self, req: &mut dyn Request) -> Result<Response> {
        let request_start = Instant::now();
        let res = self.handler.as_ref().unwrap().call(req);
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

pub fn add_custom_metadata<V: Display>(req: &mut dyn Request, key: &'static str, value: V) {
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
pub(crate) fn get_log_message(req: &dyn Request, key: &'static str) -> String {
    for (k, v) in &req.extensions().find::<CustomMetadata>().unwrap().entries {
        if key == *k {
            return v.clone();
        }
    }
    String::new()
}

struct RequestLine<'r> {
    req: &'r dyn Request,
    res: &'r Result<Response>,
    response_time: u64,
}

impl Display for RequestLine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        let (at, status) = match self.res {
            Ok(resp) => ("info", resp.status.0),
            Err(_) => ("error", 500),
        };

        line.add_field("at", at)?;
        line.add_field("method", self.req.method())?;
        line.add_quoted_field("path", FullPath(self.req))?;
        line.add_field("request_id", request_header(self.req, "X-Request-Id"))?;
        line.add_quoted_field("fwd", request_header(self.req, "X-Real-Ip"))?;
        line.add_field("service", TimeMs(self.response_time))?;
        line.add_field("status", status)?;
        line.add_quoted_field("user_agent", request_header(self.req, "User-Agent"))?;

        if let Some(metadata) = self.req.extensions().find::<CustomMetadata>() {
            for (key, value) in &metadata.entries {
                line.add_quoted_field(key, value)?;
            }
        }

        if let Some(len) = self.req.extensions().find::<u64>() {
            line.add_field("metadata_length", len)?;
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

struct FullPath<'a>(&'a dyn Request);

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
