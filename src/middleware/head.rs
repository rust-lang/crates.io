//! Middleware that proxies HEAD requests into a GET request then throws away the body

use super::prelude::*;

use crate::util::RequestProxy;
use conduit::Method;
use std::io;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct Head {
    handler: Option<Box<dyn Handler>>,
}

impl AroundMiddleware for Head {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for Head {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        if req.method() == Method::Head {
            let mut req = RequestProxy {
                other: req,
                path: None,
                method: Some(Method::Get),
            };
            self.handler
                .as_ref()
                .unwrap()
                .call(&mut req)
                .map(|r| Response {
                    body: Box::new(io::empty()),
                    ..r
                })
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
