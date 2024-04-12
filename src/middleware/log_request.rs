//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use crate::ci::CiService;
use crate::controllers::util::RequestPartsExt;
use crate::headers::XRequestId;
use crate::middleware::normalize_path::OriginalPath;
use crate::middleware::real_ip::RealIp;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Extension;
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;
use http::{Method, StatusCode, Uri};
use parking_lot::Mutex;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

const SLOW_REQUEST_THRESHOLD_MS: u128 = 1000;

#[derive(Clone, Debug)]
pub struct ErrorField(pub String);

#[derive(Clone, Debug)]
pub struct CauseField(pub String);

#[derive(axum::extract::FromRequestParts)]
pub struct RequestMetadata {
    method: Method,
    uri: Uri,
    original_path: Option<Extension<OriginalPath>>,
    real_ip: Extension<RealIp>,
    user_agent: Option<TypedHeader<UserAgent>>,
    request_id: Option<TypedHeader<XRequestId>>,
    ci_service: Option<CiService>,
}

pub struct Metadata<'a> {
    request: RequestMetadata,
    status: StatusCode,
    cause: Option<&'a CauseField>,
    error: Option<&'a ErrorField>,
    duration: Duration,
    custom_metadata: RequestLog,
}

impl Display for Metadata<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut line = LogLine::new(f);

        line.add_field("method", &self.request.method)?;

        if let Some(original_path) = &self.request.original_path {
            line.add_quoted_field("path", &original_path.deref().0)?;
        } else {
            line.add_quoted_field("path", &self.request.uri)?;
        }

        match &self.request.request_id {
            Some(header) => line.add_field("request_id", header.as_str())?,
            None => line.add_field("request_id", "")?,
        };

        line.add_quoted_field("ip", **self.request.real_ip)?;

        let response_time_in_ms = self.duration.as_millis();
        if response_time_in_ms > 0 {
            line.add_field("service", format!("{response_time_in_ms}ms"))?;
        }

        line.add_field("status", self.status.as_str())?;

        let user_agent = self.request.user_agent.as_ref();
        let user_agent = user_agent.map(|ua| ua.as_str()).unwrap_or_default();
        line.add_quoted_field("user_agent", user_agent)?;

        if self.request.original_path.is_some() {
            line.add_quoted_field("normalized_path", &self.request.uri)?;
        }

        if let Some(ci_service) = self.request.ci_service {
            line.add_quoted_field("ci", ci_service)?;
        }

        let metadata = self.custom_metadata.lock();
        for (key, value) in &*metadata {
            line.add_quoted_field(key, value)?;
        }

        if let Some(CauseField(ref cause)) = self.cause {
            line.add_quoted_field("cause", cause)?;
        }

        if let Some(ErrorField(ref error)) = self.error {
            line.add_quoted_field("error", error)?;
        }

        if response_time_in_ms > SLOW_REQUEST_THRESHOLD_MS {
            line.add_marker("SLOW REQUEST")?;
        }

        Ok(())
    }
}

pub async fn log_requests(
    request_metadata: RequestMetadata,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let start_instant = Instant::now();

    let custom_metadata = RequestLog::default();
    req.extensions_mut().insert(custom_metadata.clone());

    let response = next.run(req).await;

    let metadata = Metadata {
        request: request_metadata,
        status: response.status(),
        cause: response.extensions().get(),
        error: response.extensions().get(),
        duration: start_instant.elapsed(),
        custom_metadata,
    };

    if metadata.status.is_server_error() {
        error!(target: "http", "{metadata}");
    } else {
        info!(target: "http", "{metadata}");
    };

    response
}

#[derive(Clone, Debug, Deref, Default)]
pub struct RequestLog(Arc<Mutex<Vec<(&'static str, String)>>>);

impl RequestLog {
    pub fn add<V: Display>(&self, key: &'static str, value: V) {
        let mut metadata = self.lock();
        metadata.push((key, value.to_string()));

        sentry::configure_scope(|scope| scope.set_extra(key, value.to_string().into()));
    }
}

pub trait RequestLogExt {
    fn request_log(&self) -> &RequestLog;
}

impl<T: RequestPartsExt> RequestLogExt for T {
    fn request_log(&self) -> &RequestLog {
        self.extensions()
            .get::<RequestLog>()
            .expect("Failed to find `RequestLog` request extension")
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
