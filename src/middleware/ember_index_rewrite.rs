//! Rewrite the request path to "index.html" if the path doesn't start
//! with "/api" and the Accept header contains "html".

use super::prelude::*;

use crate::util::RequestProxy;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct EmberIndexRewrite {
    handler: Option<Box<dyn Handler>>,
}

impl Default for EmberIndexRewrite {
    fn default() -> EmberIndexRewrite {
        EmberIndexRewrite { handler: None }
    }
}

impl AroundMiddleware for EmberIndexRewrite {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for EmberIndexRewrite {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let handler = self.handler.as_ref().unwrap();
        let is_backend_path = match req.path() {
            // Special case routes used for authentication
            "/authorize" | "/authorize_url" | "/logout" => true,
            // Paths starting with `/api` are intended for the backend
            path if path.starts_with("/api") => true,
            _ => false,
        };

        if is_backend_path {
            handler.call(req)
        } else {
            // Serve static Ember page to bootstrap the frontend
            handler.call(&mut RequestProxy::rewrite_path(req, "/index.html"))
        }
    }
}
