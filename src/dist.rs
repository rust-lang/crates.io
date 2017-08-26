//! This module implements middleware to serve the compiled emberjs
//! frontend
use std::error::Error;

use conduit::{Request, Response, Handler};
use conduit_static::Static;
use conduit_middleware::AroundMiddleware;

use util::RequestProxy;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct Middleware {
    handler: Option<Box<Handler>>,
    dist: Static,
}

impl Default for Middleware {
    fn default() -> Middleware {
        Middleware {
            handler: None,
            dist: Static::new("dist"),
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
        // First, attempt to serve a static file. If we're missing a static
        // file, then keep going.
        match self.dist.call(req) {
            Ok(ref resp) if resp.status.0 == 404 => {}
            ret => return ret,
        }

        // Second, if we're requesting html, then we've only got one page so
        // serve up that page. Otherwise proxy on to the rest of the app.
        let wants_html = req.headers()
            .find("Accept")
            .map(|accept| accept.iter().any(|s| s.contains("html")))
            .unwrap_or(false);
        // If the route starts with /api, just assume they want the API
        // response. Someone is either debugging or trying to download a crate.
        let is_api_path = req.path().starts_with("/api");
        if wants_html && !is_api_path {
            self.dist.call(&mut RequestProxy {
                other: req,
                path: Some("/index.html"),
                method: None,
            })
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
