//! This module implements middleware to serve the compiled, static assets.
use std::error::Error;

use conduit::{Handler, Request, Response};
use conduit_static;
use conduit_middleware::Middleware;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Static;

impl Middleware for Static {
    fn after(&self, req: &mut Request, resp: Result<Response, Box<Error+Send>>)
                    -> Result<Response, Box<Error+Send>>
    {
        match resp {
            Ok(resp) => {
                if resp.status.0 == 404 {
                    conduit_static::Static::new("dist").call(req)
                } else {
                    Ok(resp)
                }
            }
            Err(resp) => Err(resp)
        }
    }
}
