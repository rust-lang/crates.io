//! Ensures that we returned a well formed response when we error, because civet vomits

use super::prelude::*;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct EnsureWellFormed500;

impl Middleware for EnsureWellFormed500 {
    fn after(&self, _: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        res.or_else(|_| {
            let body = "Internal Server Error";
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_LENGTH, body.len())
                .body(Box::new(body.as_bytes()) as Body)
                .map_err(box_error)
        })
    }
}
