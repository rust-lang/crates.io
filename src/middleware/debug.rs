//! Debug middleware that prints debug info to stdout

use super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Debug;

impl Middleware for Debug {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        DebugRequest.before(req)
    }

    fn after(&self, _req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        res.map(|res| {
            debug!("  <- {:?}", res.status());
            for (k, v) in res.headers().iter() {
                debug!("  <- {k} {v:?}");
            }
            res
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct DebugRequest;

impl Middleware for DebugRequest {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        debug!("  version: {:?}", req.http_version());
        debug!("  method: {:?}", req.method());
        debug!("  scheme: {:?}", req.scheme());
        debug!("  host: {:?}", req.host());
        debug!("  path: {}", req.path());
        debug!("  query_string: {:?}", req.query_string());
        debug!("  remote_addr: {:?}", req.remote_addr());
        for (k, ref v) in req.headers().iter() {
            debug!("  hdr: {}={:?}", k, v);
        }
        Ok(())
    }
}
