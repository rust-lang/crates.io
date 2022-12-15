//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::util::request_header;

use conduit::RequestExt;

use crate::headers::{XRealIp, XRequestId};
use crate::middleware::normalize_path::OriginalPath;
use axum::headers::UserAgent;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::TypedHeader;
use http::{header, Method, Request, StatusCode, Uri};
use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SLOW_REQUEST_THRESHOLD_MS: u128 = 1000;

#[derive(Default)]
pub(super) struct LogRequests();

impl Middleware for LogRequests {
    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        RequestLine::new(req, &res).log();

        res
    }
}

#[derive(axum::extract::FromRequestParts)]
pub struct RequestMetadata {
    method: Method,
    uri: Uri,
    user_agent: TypedHeader<UserAgent>,
    request_id: Option<TypedHeader<XRequestId>>,
    real_ip: Option<TypedHeader<XRealIp>>,
}

pub struct Metadata {
    request: RequestMetadata,
    status: StatusCode,
    duration: Duration,
    custom_metadata: CustomMetadata,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        // The download endpoint is our most requested endpoint by 1-2 orders of
        // magnitude. Since we pay per logged GB we try to reduce the amount of
        // bytes per log line for this endpoint.

        let is_download_endpoint = self.request.uri.path().ends_with("/download");
        let is_download_redirect = is_download_endpoint && self.status.is_redirection();

        let method = &self.request.method;
        if !is_download_redirect || method != Method::GET {
            line.add_field("method", method)?;
        }

        line.add_quoted_field("path", &self.request.uri)?;

        if !is_download_redirect {
            match &self.request.request_id {
                Some(header) => line.add_field("request_id", header.as_str())?,
                None => line.add_field("request_id", "")?,
            };
        }

        match &self.request.real_ip {
            Some(header) => line.add_quoted_field("fwd", header.as_str())?,
            None => line.add_quoted_field("fwd", "")?,
        };

        let response_time_in_ms = self.duration.as_millis();
        if !is_download_redirect || response_time_in_ms > 0 {
            line.add_field("service", format!("{}ms", response_time_in_ms))?;
        }

        if !is_download_redirect {
            line.add_field("status", self.status.as_str())?;
        }

        line.add_quoted_field("user_agent", self.request.user_agent.as_str())?;

        if let Ok(metadata) = self.custom_metadata.lock() {
            for (key, value) in &*metadata {
                line.add_quoted_field(key, value)?;
            }
        }

        // if let Err(err) = self.res {
        //     line.add_quoted_field("error", err)?;
        // }

        if response_time_in_ms > SLOW_REQUEST_THRESHOLD_MS {
            line.add_marker("SLOW REQUEST")?;
        }

        Ok(())
    }
}

pub async fn log_requests<B>(
    request_metadata: RequestMetadata,
    mut req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let start_instant = Instant::now();

    let custom_metadata = CustomMetadata::default();
    req.extensions_mut().insert(custom_metadata.clone());

    let response = next.run(req).await;

    let metadata = Metadata {
        request: request_metadata,
        status: response.status(),
        duration: start_instant.elapsed(),
        custom_metadata,
    };
    debug!(target: "axum", "{metadata}");

    response
}

#[derive(Clone, Debug, Deref, Default)]
pub struct CustomMetadata(Arc<Mutex<Vec<(&'static str, String)>>>);

pub fn add_custom_metadata<V: Display>(req: &dyn RequestExt, key: &'static str, value: V) {
    if let Some(metadata) = req.extensions().get::<CustomMetadata>() {
        if let Ok(mut metadata) = metadata.lock() {
            metadata.push((key, value.to_string()));
        }
    }

    sentry::configure_scope(|scope| scope.set_extra(key, value.to_string().into()));
}

#[cfg(test)]
pub(crate) fn get_log_message(req: &dyn RequestExt, key: &'static str) -> String {
    // Unwrap shouldn't panic as no other code has access to the private struct to remove it
    if let Some(metadata) = req.extensions().get::<CustomMetadata>() {
        if let Ok(metadata) = metadata.lock() {
            for (k, v) in &*metadata {
                if key == *k {
                    return v.clone();
                }
            }
        }
    }

    panic!("expected log message for {} not found", key);
}

struct RequestLine<'r> {
    req: &'r dyn RequestExt,
    res: &'r AfterResult,
    status: StatusCode,
}

impl<'a> RequestLine<'a> {
    fn new(request: &'a dyn RequestExt, response: &'a AfterResult) -> Self {
        let status = response.as_ref().map(|res| res.status());
        let status = status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        RequestLine {
            req: request,
            res: response,
            status,
        }
    }

    fn log(&self) {
        if self.status.is_server_error() {
            error!(target: "http", "{self}");
        } else {
            info!(target: "http", "{self}");
        };
    }
}

impl Display for RequestLine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        // The download endpoint is our most requested endpoint by 1-2 orders of
        // magnitude. Since we pay per logged GB we try to reduce the amount of
        // bytes per log line for this endpoint.

        let is_download_endpoint = self.req.path().ends_with("/download");
        let is_download_redirect = is_download_endpoint && self.status.is_redirection();

        let method = self.req.method();
        if !is_download_redirect || method != Method::GET {
            line.add_field("method", method)?;
        }

        line.add_quoted_field("path", FullPath(self.req))?;

        if !is_download_redirect {
            line.add_field("request_id", request_header(self.req, "x-request-id"))?;
        }

        line.add_quoted_field("fwd", request_header(self.req, "x-real-ip"))?;

        let response_time_in_ms = self.req.elapsed().as_millis();
        if !is_download_redirect || response_time_in_ms > 0 {
            line.add_field("service", format!("{}ms", response_time_in_ms))?;
        }

        if !is_download_redirect {
            line.add_field("status", self.status.as_str())?;
        }

        line.add_quoted_field("user_agent", request_header(self.req, header::USER_AGENT))?;

        if let Some(mutex) = self.req.extensions().get::<CustomMetadata>() {
            if let Ok(metadata) = mutex.lock() {
                for (key, value) in &*metadata {
                    line.add_quoted_field(key, value)?;
                }
            }
        };

        if let Err(err) = self.res {
            line.add_quoted_field("error", err)?;
        }

        if response_time_in_ms > SLOW_REQUEST_THRESHOLD_MS {
            line.add_marker("SLOW REQUEST")?;
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
