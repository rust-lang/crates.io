use std::io;
use std::fmt::Show;

use conduit::{mod, Request, Response, Handler};
use conduit_middleware::AroundMiddleware;

use util::RequestProxy;

pub struct Head {
    handler: Option<Box<Handler + Send + Sync>>,
}

impl Head {
    pub fn new() -> Head {
        Head { handler: None }
    }
}

impl AroundMiddleware for Head {
    fn with_handler(&mut self, handler: Box<Handler + Send + Sync>) {
        self.handler = Some(handler);
    }
}

impl Handler for Head {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show + 'static>> {
        if req.method() == conduit::Head {
            let mut req = RequestProxy {
                other: req,
                path: None,
                method: Some(conduit::Get),
            };
            self.handler.as_ref().unwrap().call(&mut req).map(|r| {
                Response {
                    body: box io::util::NullReader,
                    ..r
                }
            })
        } else {
            self.handler.as_ref().unwrap().call(req)
        }
    }
}
