//! Middleware that blocks requests with no user-agent header
//!
//! By default the middleware will treat "" and "Amazon CloudFront" as a missing user-agent. To
//! change the 2nd value, set `WEB_CDN_USER_AGENT` to the appropriate string. To disable the CDN
//! check, set `WEB_CDN_USER_AGENT` to the empty string.
//!
//! Requests to the download endpoint are always allowed, to support versions of cargo older than
//! 0.17 (released alongside rustc 1.17).

use super::prelude::*;
use crate::middleware::app::RequestApp;

use crate::util::request_header;

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
    fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
        let cdn_user_agent = &req.app().config.cdn_user_agent;

        let agent = request_header(req, header::USER_AGENT);
        let has_user_agent = !agent.is_empty() && agent != cdn_user_agent;
        let is_download = req.path().ends_with("download");
        if !has_user_agent && !is_download {
            add_custom_metadata(req, "cause", "no user agent");
            let body = format!(
                include_str!("no_user_agent_message.txt"),
                request_header(req, "x-request-id"),
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
