//! Middleware that blocks requests with no user-agent header

use super::prelude::*;

use crate::util::request_header;

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
    fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
        let agent = request_header(req, header::USER_AGENT);
        let has_user_agent = agent != "" && agent != "Amazon CloudFront";
        let is_download = req.path().ends_with("download");
        if !has_user_agent && !is_download {
            super::log_request::add_custom_metadata(req, "cause", "no user agent");
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
