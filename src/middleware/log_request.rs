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
use http::{Method, Uri};
use parking_lot::Mutex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Instant;
use tracing::Level;

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

pub async fn log_requests(
    request_metadata: RequestMetadata,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let start_instant = Instant::now();

    let custom_metadata = RequestLog::default();
    req.extensions_mut().insert(custom_metadata.clone());

    let response = next.run(req).await;

    let duration = start_instant.elapsed();

    let method = &request_metadata.method;
    let url = request_metadata
        .original_path
        .as_ref()
        .map(|p| Cow::Borrowed(&p.0 .0))
        .unwrap_or_else(|| Cow::Owned(request_metadata.uri.to_string()));

    let status = response.status();

    let custom_metadata = {
        let metadata = custom_metadata.lock();
        let metadata = metadata.iter().cloned().collect::<HashMap<&str, String>>();
        serde_json::to_string(&metadata).unwrap_or_default()
    };

    event!(
        target: "http",
        Level::INFO,
        duration = duration.as_nanos(),
        network.client.ip = %**request_metadata.real_ip,
        http.method = %method,
        http.url = %url,
        http.request_id = %request_metadata.request_id.as_ref().map(|h| h.as_str()).unwrap_or_default(),
        http.useragent = %request_metadata.user_agent.as_ref().map(|h| h.as_str()).unwrap_or_default(),
        http.status_code = status.as_u16(),
        cause = response.extensions().get::<CauseField>().map(|e| e.0.as_str()).unwrap_or_default(),
        error.message = response.extensions().get::<ErrorField>().map(|e| e.0.as_str()).unwrap_or_default(),
        ci = %request_metadata.ci_service.map(|ci| ci.to_string()).unwrap_or_default(),
        %custom_metadata,
        "{method} {url} → {status} ({duration:?})",
    );

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
