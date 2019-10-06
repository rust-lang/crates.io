use std::cmp;
use std::collections::HashMap;
use std::io::Cursor;

use conduit::Response;
use serde::Serialize;

pub use self::errors::ChainError;
pub use self::errors::{bad_request, human, internal, internal_error, CargoError, CargoResult};
pub use self::io_util::{read_fill, read_le_u32, LimitErrorReader};
pub use self::request_helpers::*;
pub use self::request_proxy::RequestProxy;

pub mod errors;
mod io_util;
mod request_helpers;
mod request_proxy;
pub mod rfc3339;

pub fn json_response<T: Serialize>(t: &T) -> Response {
    let json = serde_json::to_string(t).unwrap();
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        vec!["application/json; charset=utf-8".to_string()],
    );
    headers.insert("Content-Length".to_string(), vec![json.len().to_string()]);
    Response {
        status: (200, "OK"),
        headers,
        body: Box::new(Cursor::new(json.into_bytes())),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Maximums {
    pub max_upload_size: u64,
    pub max_unpack_size: u64,
}

impl Maximums {
    pub fn new(
        krate_max_upload: Option<i32>,
        app_max_upload: u64,
        app_max_unpack: u64,
    ) -> Maximums {
        let max_upload_size = krate_max_upload.map(|m| m as u64).unwrap_or(app_max_upload);
        let max_unpack_size = cmp::max(app_max_unpack, max_upload_size);
        Maximums {
            max_upload_size,
            max_unpack_size,
        }
    }
}
