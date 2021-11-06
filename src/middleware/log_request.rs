//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::util::request_header;

use conduit::{header, RequestExt, StatusCode};

use crate::middleware::normalize_path::OriginalPath;
use crate::middleware::response_timing::ResponseTime;
use std::fmt::{self, Display, Formatter};

const SLOW_REQUEST_THRESHOLD_MS: u64 = 1000;

#[derive(Default)]
pub(super) struct LogRequests();

impl Middleware for LogRequests {
    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        println!("{}", RequestLine { req, res: &res });

        res
    }
}

struct CustomMetadata {
    entries: Vec<(&'static str, String)>,
}

pub fn add_custom_metadata<V: Display>(req: &mut dyn RequestExt, key: &'static str, value: V) {
    if let Some(metadata) = req.mut_extensions().get_mut::<CustomMetadata>() {
        metadata.entries.push((key, value.to_string()));
    } else {
        let mut metadata = CustomMetadata {
            entries: Vec::new(),
        };
        metadata.entries.push((key, value.to_string()));
        req.mut_extensions().insert(metadata);
    }

    sentry::configure_scope(|scope| scope.set_extra(key, value.to_string().into()));
}

#[cfg(test)]
pub(crate) fn get_log_message(req: &dyn RequestExt, key: &'static str) -> String {
    // Unwrap shouldn't panic as no other code has access to the private struct to remove it
    for (k, v) in &req.extensions().get::<CustomMetadata>().unwrap().entries {
        if key == *k {
            return v.clone();
        }
    }
    panic!("expected log message for {} not found", key);
}

struct RequestLine<'r> {
    req: &'r dyn RequestExt,
    res: &'r AfterResult,
}

impl Display for RequestLine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        let status = self.res.as_ref().map(|res| res.status());
        let status = status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let at = if status.is_server_error() {
            "error"
        } else {
            "info"
        };

        line.add_field("at", at)?;
        line.add_field("method", self.req.method())?;
        line.add_quoted_field("path", FullPath(self.req))?;

        // The request_id is not logged for successful download requests
        if !(self.req.path().ends_with("/download")
            && self
                .res
                .as_ref()
                .ok()
                .map(|ok| ok.status().is_redirection())
                == Some(true))
        {
            line.add_field("request_id", request_header(self.req, "x-request-id"))?;
        }

        line.add_quoted_field("fwd", request_header(self.req, "x-real-ip"))?;

        let response_time = self.req.extensions().get::<ResponseTime>();
        if let Some(response_time) = response_time {
            line.add_field("service", response_time)?;
        }
        line.add_field("status", status.as_str())?;
        line.add_quoted_field("user_agent", request_header(self.req, header::USER_AGENT))?;

        if let Some(metadata) = self.req.extensions().get::<CustomMetadata>() {
            for (key, value) in &metadata.entries {
                line.add_quoted_field(key, value)?;
            }
        }

        if let Err(err) = self.res {
            line.add_quoted_field("error", err)?;
        }

        if let Some(response_time) = response_time {
            if response_time.as_millis() > SLOW_REQUEST_THRESHOLD_MS {
                line.add_marker("SLOW REQUEST")?;
            }
        }

        Ok(())
    }
}

struct FullPath<'a>(&'a dyn RequestExt);

impl<'a> Display for FullPath<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let request = self.0;

        let original_path = request.extensions().get::<OriginalPath>();
        let path = original_path
            .map(|p| p.0.as_str())
            .unwrap_or_else(|| request.path());

        write!(f, "{}", path)?;

        if let Some(q_string) = request.query_string() {
            write!(f, "?{}", q_string)?;
        }
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
