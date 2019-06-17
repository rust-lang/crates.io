//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent

use super::prelude::*;
use crate::util::request_header;
use conduit::Request;
use std::fmt;
use std::time::Instant;

#[allow(missing_debug_implementations)] // We can't
#[derive(Default)]
pub struct LogRequests {
    handler: Option<Box<dyn Handler>>,
}

impl AroundMiddleware for LogRequests {
    fn with_handler(&mut self, handler: Box<dyn Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for LogRequests {
    fn call(&self, req: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let request_start = Instant::now();
        let res = self.handler.as_ref().unwrap().call(req);
        let (level, response_code) = match res {
            Ok(ref r) => ("info", r.status.0),
            Err(_) => ("error", 500),
        };
        let response_time = request_start.elapsed();
        let response_time =
            response_time.as_secs() * 1000 + u64::from(response_time.subsec_nanos()) / 1_000_000;

        print!(
            "at={level} method={method} path=\"{path}\" \
             request_id={request_id} fwd=\"{ip}\" service={time_ms}ms \
             status={status} user_agent=\"{user_agent}\"",
            level = level,
            method = req.method(),
            path = FullPath(req),
            ip = request_header(req, "X-Real-Ip"),
            time_ms = response_time,
            user_agent = request_header(req, "User-Agent"),
            request_id = request_header(req, "X-Request-Id"),
            status = response_code,
        );

        if let Err(ref e) = res {
            print!(" error=\"{}\"", e.description());
        }

        if response_time > 1000 {
            print!(" SLOW REQUEST");
        }

        println!();

        res
    }
}

struct FullPath<'a>(&'a dyn Request);

impl<'a> fmt::Display for FullPath<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.path())?;
        if let Some(q_string) = self.0.query_string() {
            write!(f, "?{}", q_string)?;
        }
        Ok(())
    }
}
