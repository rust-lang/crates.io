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
            println!("  <- {:?}", res.status());
            for (k, v) in res.headers().iter() {
                println!("  <- {} {:?}", k, v);
            }
            res
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct DebugRequest;

impl Middleware for DebugRequest {
    fn before(&self, req: &mut dyn RequestExt) -> BeforeResult {
        println!("  version: {:?}", req.http_version());
        println!("  method: {:?}", req.method());
        println!("  scheme: {:?}", req.scheme());
        println!("  host: {:?}", req.host());
        println!("  path: {}", req.path());
        println!("  query_string: {:?}", req.query_string());
        println!("  remote_addr: {:?}", req.remote_addr());
        for (k, ref v) in req.headers().iter() {
            println!("  hdr: {}={:?}", k, v);
        }
        Ok(())
    }
}
