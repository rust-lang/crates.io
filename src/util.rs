use std::cmp;

pub use self::bytes_request::BytesRequest;
pub use self::io_util::{read_fill, read_le_u32};
pub use self::request_helpers::*;

mod bytes_request;
pub mod diesel;
pub mod errors;
mod io_util;
mod request_helpers;
pub mod rfc3339;
pub mod token;
pub mod tracing;

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
