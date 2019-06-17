//! Middleware that blocks requests from a list of given IPs

use super::prelude::*;

use std::collections::HashMap;
use std::io::Cursor;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct BlockIps {
    ips: Vec<String>,
    handler: Option<Box<dyn Handler>>,
}

impl BlockIps {
    pub fn new(ips: Vec<String>) -> Self {
        Self { ips, handler: None }
    }
}

impl AroundMiddleware for BlockIps {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for BlockIps {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let has_blocked_ip = req
            .headers()
            .find("X-Real-Ip")
            .unwrap()
            .iter()
            .any(|ip| self.ips.iter().any(|v| v == ip));
        if has_blocked_ip {
            let body = format!(
                "We are unable to process your request at this time. \
                 This usually means that you are in violation of our crawler \
                 policy (https://crates.io/policies#crawlers). \
                 Please open an issue at https://github.com/rust-lang/crates.io \
                 or email help@crates.io \
                 and provide the request id {}",
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
