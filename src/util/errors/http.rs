use std::fmt;

use conduit::Response;

use super::{AppError, Bad, StringError};
use crate::util::json_response;

#[derive(Debug)]
pub(super) struct ServerError(pub(super) String);

impl AppError for ServerError {
    fn description(&self) -> &str {
        self.0.as_ref()
    }

    fn response(&self) -> Option<Response> {
        let mut response = json_response(&Bad {
            errors: vec![StringError {
                detail: self.0.clone(),
            }],
        });
        response.status = (500, "Internal Server Error");
        Some(response)
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
