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
    pub max_upload_size: u32,
    pub max_unpack_size: u64,
}

impl Maximums {
    pub fn new(
        krate_max_upload: Option<i32>,
        app_max_upload: u32,
        app_max_unpack: u64,
    ) -> Maximums {
        let krate_max_upload = krate_max_upload.and_then(|m| u32::try_from(m).ok());
        let max_upload_size = krate_max_upload.unwrap_or(app_max_upload);
        let max_unpack_size = cmp::max(app_max_unpack, max_upload_size as u64);
        Maximums {
            max_upload_size,
            max_unpack_size,
        }
    }
}
