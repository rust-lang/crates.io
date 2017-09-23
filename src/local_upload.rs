//! This module implements middleware to serve crates and readmes
//! from the `local_uploads/` directory. This is only used in
//! development environments.
use std::error::Error;

use conduit::{Handler, Request, Response};
use conduit_static::Static;
use conduit_middleware::AroundMiddleware;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct Middleware {
    handler: Option<Box<Handler>>,
    local_uploads: Static,
}

impl Default for Middleware {
    fn default() -> Middleware {
        Middleware {
            handler: None,
            local_uploads: Static::new("local_uploads"),
        }
    }
}

impl AroundMiddleware for Middleware {
    fn with_handler(&mut self, handler: Box<Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for Middleware {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        match self.local_uploads.call(req) {
            Ok(ref resp) if resp.status.0 == 404 => {}
            ret => return ret,
        }

        self.handler.as_ref().unwrap().call(req)
    }
}
