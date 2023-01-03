use super::prelude::*;
use crate::util::errors::{forbidden, internal, AppError, AppResult};
use conduit_router::RequestParams;

/// The Origin header (https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Origin)
/// is sent with CORS requests and POST requests, and indicates where the request comes from.
/// We don't want to accept authenticated requests that originated from other sites, so this
/// function returns an error if the Origin header doesn't match what we expect "this site" to
/// be: https://crates.io in production, or http://localhost:port/ in development.
pub fn verify_origin(req: &dyn RequestExt) -> AppResult<()> {
    let headers = req.headers();
    let allowed_origins = &req.app().config.allowed_origins;

    let bad_origin = headers
        .get_all(header::ORIGIN)
        .iter()
        .find(|value| !allowed_origins.contains(value));

    if let Some(bad_origin) = bad_origin {
        let error_message =
            format!("only same-origin requests can be authenticated. got {bad_origin:?}");
        return Err(internal(&error_message).chain(forbidden()));
    }
    Ok(())
}

pub trait RequestParamExt<'a> {
    fn param(self, key: &str) -> Option<&'a str>;
}

impl<'a> RequestParamExt<'a> for &'a (dyn RequestExt + 'a) {
    fn param(self, key: &str) -> Option<&'a str> {
        self.params().find(key)
    }
}
