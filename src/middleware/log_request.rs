//! Log all requests in a format similar to Heroku's router, but with additional
//! information that we care about like User-Agent and Referer

use conduit::Request;
use std::time::Instant;
use super::prelude::*;
use util::request_header;

#[allow(missing_debug_implementations)] // We can't
#[derive(Default)]
pub struct LogRequests {
    handler: Option<Box<Handler>>,
}

impl AroundMiddleware for LogRequests {
    fn with_handler(&mut self, handler: Box<Handler>) {
        self.handler = Some(handler);
    }
}

impl Handler for LogRequests {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error + Send>> {
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
             status={status} user_agent=\"{user_agent}\" referer=\"{referer}\"",
            level = level,
            method = req.method(),
            path = req.path(),
            ip = request_header(req, "X-Forwarded-For"),
            time_ms = response_time,
            user_agent = request_header(req, "User-Agent"),
            referer = request_header(req, "Referer"), // sic
            request_id = request_header(req, "X-Request-Id"),
            status = response_code,
        );

        if let Err(ref e) = res {
            print!(" error=\"{}\"", e.description());
        }

        println!();

        res
    }
}
