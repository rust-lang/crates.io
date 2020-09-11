use std::cmp;

use conduit::{header, Body, Response};
use rand::{distributions::Uniform, rngs::OsRng, Rng};
use serde::Serialize;

pub use self::io_util::{read_fill, read_le_u32, LimitErrorReader};
pub use self::request_helpers::*;
pub use self::request_proxy::RequestProxy;

pub mod errors;
mod io_util;
mod request_helpers;
mod request_proxy;
pub mod rfc3339;

pub type AppResponse = Response<conduit::Body>;
pub type EndpointResult = Result<AppResponse, Box<dyn errors::AppError>>;

/// Serialize a value to JSON and build a status 200 Response
///
/// This helper sets appropriate values for `Content-Type` and `Content-Length`.
///
/// # Panics
///
/// This function will panic if serialization fails.
pub fn json_response<T: Serialize>(t: &T) -> AppResponse {
    let json = serde_json::to_string(t).unwrap();
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(header::CONTENT_LENGTH, json.len())
        .body(Body::from_vec(json.into_bytes()))
        .unwrap() // Header values are well formed, so should not panic
}

pub fn generate_secure_alphanumeric_string(len: usize) -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    OsRng
        .sample_iter(Uniform::from(0..CHARS.len()))
        .map(|idx| CHARS[idx] as char)
        .take(len)
        .collect()
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
