use std::cmp;

use conduit::{header, Body, Response};
use serde::Serialize;

pub use self::io_util::{read_fill, read_le_u32, LimitErrorReader};
pub use self::request_helpers::*;
pub use self::request_proxy::RequestProxy;

pub mod errors;
mod io_util;
mod request_helpers;
mod request_proxy;
pub mod rfc3339;
pub(crate) mod token;

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

#[macro_export]
macro_rules! pg_enum {
    (
        $vis:vis enum $name:ident {
            $($item:ident = $int:expr,)*
        }
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, FromSqlRow, AsExpression)]
        #[sql_type = "diesel::sql_types::Integer"]
        #[serde(rename_all = "snake_case")]
        #[repr(i32)]
        $vis enum $name {
            $($item = $int,)*
        }

        impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg> for $name {
            fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
                match <i32 as diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg>>::from_sql(bytes)? {
                    $($int => Ok(Self::$item),)*
                    n => Err(format!("unknown value: {}", n).into()),
                }
            }
        }

        impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::pg::Pg> for $name {
            fn to_sql<W: std::io::Write>(
                &self,
                out: &mut diesel::serialize::Output<'_, W, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                diesel::serialize::ToSql::<diesel::sql_types::Integer, diesel::pg::Pg>::to_sql(&(*self as i32), out)
            }
        }

        impl $name {
            #[allow(unused)]
            $vis const VARIANTS: &'static [Self] = &[$($name::$item),*];
        }
    }
}
