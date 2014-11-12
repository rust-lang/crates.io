use std::fmt::Show;
use std::os;

use conduit::{Handler, Request, Response};
use conduit_middleware::AroundMiddleware;

use util::RequestUtils;

pub struct LaunchGuard {
    enabled: bool,
    handler: Option<Box<Handler + Send + Sync>>,
}

impl LaunchGuard {
    pub fn new() -> LaunchGuard {
        LaunchGuard {
            handler: None,
            enabled: os::getenv("NO_LAUNCH_GUARD").is_none(),
        }
    }
}

impl Handler for LaunchGuard {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show + 'static>> {
        match req.headers().find("Host") {
            Some(v) => {
                if self.enabled && v.iter().any(|s| s.contains("crates.io")) {
                    return Ok(req.redirect("http://doc.crates.io/".to_string()))
                }
            }
            None => {}
        }
        self.handler.as_ref().unwrap().call(req)
    }
}

impl AroundMiddleware for LaunchGuard {
    fn with_handler(&mut self, handler: Box<Handler + Send + Sync>) {
        self.handler = Some(handler);
    }
}
