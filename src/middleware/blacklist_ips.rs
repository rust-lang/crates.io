//! Middleware that blocks requests from a list of given IPs

use super::prelude::*;

use std::io::Cursor;
use std::collections::HashMap;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct BlockIps {
    ips: Vec<String>,
    handler: Option<Box<Handler>>,
}

impl BlockIps {
    pub fn new(ips: Vec<String>) -> Self {
        Self { ips, handler: None }
    }
}

impl AroundMiddleware for BlockIps {
    fn with_handler(&mut self, handler: Box<Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for BlockIps {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        let has_blacklisted_ip = req.headers()
            .find("X-Forwarded-For")
            .unwrap()
            .iter()
            .any(|v| v.split(", ").any(|ip| self.ips.iter().any(|x| x == ip)));
        if has_blacklisted_ip {
            let body = format!(
                "We are unable to process your request at this time. \
                 Please open an issue at https://github.com/rust-lang/crates.io \
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
