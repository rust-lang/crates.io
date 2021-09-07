use super::prelude::*;
use crate::util::request_header;

use conduit::RequestExt;

use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct ResponseTiming();

pub struct ResponseTime(u64);

impl ResponseTime {
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

impl Display for ResponseTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)?;
        f.write_str("ms")?;
        Ok(())
    }
}

impl Middleware for ResponseTiming {
    fn after(&self, req: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        let response_time =
            if let Ok(start_ms) = request_header(req, "x-request-start").parse::<u128>() {
                let current_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went way backwards")
                    .as_millis();

                if current_ms > start_ms {
                    // The result cannot be negative
                    current_ms - start_ms
                } else {
                    // Because our nginx proxy and app run on the same dyno in production, we
                    // shouldn't have to worry about clock drift. But if something goes wrong,
                    // calculate the response time based on when the request reached this app.
                    fallback_response_time(req)
                }
            } else {
                // X-Request-Start header couldn't be parsed.
                // We are probably running locally and not behind nginx.
                fallback_response_time(req)
            };

        // This will only trucate for requests lasting > 500 million years
        let response_time = response_time as u64;

        req.mut_extensions().insert(ResponseTime(response_time));

        res
    }
}

/// Calculate the response time based on when the request reached the in-app web server.
///
/// This serves as a fallback in case the `X-Request-Start` header is missing or invalid.
fn fallback_response_time(req: &mut dyn RequestExt) -> u128 {
    req.elapsed().as_millis()
}
