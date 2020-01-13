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
    fn call(&self, req: &mut dyn Request) -> Result<Response> {
        let request_start = Instant::now();
        let res = self.handler.as_ref().unwrap().call(req);
        let (level, response_code) = match res {
            Ok(ref r) => ("info", r.status.0),
            Err(_) => ("error", 500),
        };
        let response_time = request_start.elapsed();
        let response_time =
            response_time.as_secs() * 1000 + u64::from(response_time.subsec_nanos()) / 1_000_000;

        let metadata_length = req
            .extensions()
            .find::<u64>()
            .map_or(String::new(), |l| format!(" metadata_length={}", l));

        let slow_request = if response_time > 1000 {
            " SLOW REQUEST"
        } else {
            ""
        };

        let error = if let Err(ref e) = res {
            format!(" error=\"{}\"", e)
        } else {
            String::new()
        };

        println!(
            "at={level} method={method} path=\"{path}\" \
             request_id={request_id} fwd=\"{ip}\" service={time_ms}ms \
             status={status} user_agent=\"{user_agent}\"\
             {metadata_length}{error}{slow_request}",
            level = level,
            method = req.method(),
            path = FullPath(req),
            ip = request_header(req, "X-Real-Ip"),
            time_ms = response_time,
            user_agent = request_header(req, "User-Agent"),
            request_id = request_header(req, "X-Request-Id"),
            status = response_code,
            metadata_length = metadata_length,
            error = error,
            slow_request = slow_request,
        );

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
