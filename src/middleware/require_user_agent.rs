//! Middleware that blocks requests with no user-agent header

use super::prelude::*;

use crate::util::request_header;
use std::collections::HashMap;
use std::io::Cursor;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct RequireUserAgent {
    handler: Option<Box<dyn Handler>>,
}

impl AroundMiddleware for RequireUserAgent {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for RequireUserAgent {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let has_user_agent = request_header(req, "User-Agent") != "";
        let is_download = req.path().ends_with("download");
        if !has_user_agent && !is_download {
            let body = format!(
                include_str!("no_user_agent_message.txt"),
                request_header(req, "X-Request-Id"),
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
