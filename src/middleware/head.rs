//! Middleware that proxies HEAD requests into a GET request then throws away the body

use super::prelude::*;

use crate::util::RequestProxy;
use conduit::Method;

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
    fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
        if req.method() == Method::HEAD {
            let mut req = RequestProxy::rewrite_method(req, Method::GET);
            self.handler.as_ref().unwrap().call(&mut req).map(|mut r| {
                *r.body_mut() = Body::empty();
                r
            })
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
