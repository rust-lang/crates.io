//! Ensures that we returned a well formed response when we error, because civet vomits

use super::prelude::*;

use std::collections::HashMap;

// Can't derive debug because of Handler.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct EnsureWellFormed500;

impl Middleware for EnsureWellFormed500 {
    fn after(&self, _: &mut dyn Request, res: Result<Response>) -> Result<Response> {
        res.or_else(|_| {
            let body = "Internal Server Error";
            let mut headers = HashMap::new();
            headers.insert("Content-Length".to_string(), vec![body.len().to_string()]);
            Ok(Response {
                status: (500, "Internal Server Error"),
                headers,
                body: Box::new(body.as_bytes()),
            })
        })
    }
}
