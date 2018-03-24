//! This module implements middleware to serve crates and readmes
//! from the `local_uploads/` directory. This is only used in
//! development environments.
use super::prelude::*;

use conduit_static::Static;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct LocalUpload {
    handler: Option<Box<Handler>>,
    local_uploads: Static,
}

impl Default for LocalUpload {
    fn default() -> LocalUpload {
        LocalUpload {
            handler: None,
            local_uploads: Static::new("local_uploads"),
        }
    }
}

impl AroundMiddleware for LocalUpload {
    fn with_handler(&mut self, handler: Box<Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for LocalUpload {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
        match self.local_uploads.call(req) {
            Ok(ref resp) if resp.status.0 == 404 => {}
            ret => return ret,
        }

        self.handler.as_ref().unwrap().call(req)
    }
}
