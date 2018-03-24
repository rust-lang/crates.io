//! Middleware that proxies HEAD requests into a GET request then throws away the body

use super::prelude::*;

use std::io;
use conduit::Method;
use util::RequestProxy;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct Head {
    handler: Option<Box<Handler>>,
}

impl AroundMiddleware for Head {
    fn with_handler(&mut self, handler: Box<Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for Head {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
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
