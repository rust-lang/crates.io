//! Middleware that blocks requests if a header matches the given list
//!
//! To use, set the `BLOCKED_TRAFFIC` environment variable to a comma-separated list of pairs
//! containing a header name, an equals sign, and the name of another environment variable that
//! contains the values of that header that should be blocked. For example, set `BLOCKED_TRAFFIC`
//! to `User-Agent=BLOCKED_UAS,X-Real-Ip=BLOCKED_IPS`, `BLOCKED_UAS` to `curl/7.54.0,cargo 1.36.0
//! (c4fcfb725 2019-05-15)`, and `BLOCKED_IPS` to `192.168.0.1,127.0.0.1` to block requests from
//! the versions of curl or Cargo specified or from either of the IPs (values are nonsensical
//! examples). Values of the headers must match exactly.

use super::prelude::*;
use crate::App;
use std::sync::Arc;

#[derive(Default)]
pub struct BlockTraffic {
    header_name: String,
    blocked_values: Vec<String>,
    handler: Option<Box<dyn Handler>>,
}

impl BlockTraffic {
    pub fn new(header_name: String, blocked_values: Vec<String>) -> Self {
        Self {
            header_name,
            blocked_values,
            handler: None,
        }
    }
}

impl AroundMiddleware for BlockTraffic {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for BlockTraffic {
    fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
        let app = req.extensions().get::<Arc<App>>().expect("Missing app");
        let domain_name = app.config.domain_name.clone();

        let has_blocked_value = req
            .headers()
            .get_all(&self.header_name)
            .iter()
            .map(|val| val.to_str().unwrap_or_default())
            .any(|value| self.blocked_values.iter().any(|v| v == value));
        if has_blocked_value {
            let cause = format!("blocked due to contents of header {}", self.header_name);
            add_custom_metadata(req, "cause", cause);
            let body = format!(
                "We are unable to process your request at this time. \
                 This usually means that you are in violation of our crawler \
                 policy (https://{}/policies#crawlers). \
                 Please open an issue at https://github.com/rust-lang/crates.io \
                 or email help@crates.io \
                 and provide the request id {}",
                domain_name,
                // Heroku should always set this header
                req.headers()
                    .get("x-request-id")
                    .map(|val| val.to_str().unwrap_or_default())
                    .unwrap_or_default()
            );

            Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header(header::CONTENT_LENGTH, body.len())
                .body(Body::from_vec(body.into_bytes()))
                .map_err(box_error)
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
