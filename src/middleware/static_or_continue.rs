//! This module implements middleware to serve static files from the
//! specified directory.
use super::prelude::*;

use conduit_static::Static;

// Can't derive debug because of Handler and Static.
#[allow(missing_debug_implementations)]
pub struct StaticOrContinue {
    fallback_handler: Option<Box<dyn Handler>>,
    static_handler: Static,
}

impl StaticOrContinue {
    pub fn new(directory: &str) -> StaticOrContinue {
        StaticOrContinue {
            fallback_handler: None,
            static_handler: Static::new(directory),
        }
    }
}

impl AroundMiddleware for StaticOrContinue {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.fallback_handler = Some(handler);
    }
}

impl Handler for StaticOrContinue {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        match self.static_handler.call(req) {
            Ok(ref resp) if resp.status.0 == 404 => {}
            ret => return ret,
        }

        self.fallback_handler.as_ref().unwrap().call(req)
    }
}
