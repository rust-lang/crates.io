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

use std::collections::HashMap;
use std::io::Cursor;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
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
    fn call(&self, req: &mut dyn Request) -> Result<Response> {
        let has_blocked_value = req
            .headers()
            .find(&self.header_name)
            .unwrap_or_default()
            .iter()
            .any(|value| self.blocked_values.iter().any(|v| v == value));
        if has_blocked_value {
            let body = format!(
                "We are unable to process your request at this time. \
                 This usually means that you are in violation of our crawler \
                 policy (https://crates.io/policies#crawlers). \
                 Please open an issue at https://github.com/rust-lang/crates.io \
                 or email help@crates.io \
                 and provide the request id {}",
                // Heroku should always set this header
                req.headers().find("X-Request-Id").unwrap()[0]
            );
            let mut headers = HashMap::new();
            headers.insert("Content-Length".to_string(), vec![body.len().to_string()]);
            Ok(Response {
                status: (403, "Forbidden"),
                headers,
                body: Box::new(Cursor::new(body.into_bytes())),
            })
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
