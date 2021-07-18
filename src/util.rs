use std::cmp;

pub use self::bytes_request::BytesRequest;
pub use self::io_util::{read_fill, read_le_u32};
pub use self::request_helpers::*;

mod bytes_request;
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

#[macro_export]
macro_rules! pg_enum {
    (
        $vis:vis enum $name:ident {
            $($item:ident = $int:expr,)*
        }
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, FromSqlRow, AsExpression)]
        #[diesel(sql_type = diesel::sql_types::Integer)]
        #[serde(rename_all = "snake_case")]
        #[repr(i32)]
        $vis enum $name {
            $($item = $int,)*
        }

        impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg> for $name {
            fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
                match <i32 as diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg>>::from_sql(bytes)? {
                    $($int => Ok(Self::$item),)*
                    n => Err(format!("unknown value: {}", n).into()),
                }
            }
        }

        impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::pg::Pg> for $name {
            fn to_sql(
                &self,
                out: &mut diesel::serialize::Output<'_, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                diesel::serialize::ToSql::<diesel::sql_types::Integer, diesel::pg::Pg>::to_sql(&(*self as i32), &mut out.reborrow())
            }
        }
    }
}
