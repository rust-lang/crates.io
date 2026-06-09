#[macro_export]
macro_rules! pg_enum {
    (
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $($item:ident = $int:expr,)*
        }
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, diesel::FromSqlRow, diesel::AsExpression)]
        #[diesel(sql_type = diesel::sql_types::Integer)]
        #[serde(rename_all = "snake_case")]
        #[repr(i32)]
        $(#[$meta])*
        $vis enum $name {
            $($item = $int,)*
        }

        impl $name {
            $vis const VARIANTS: &'static [$name] = &[$($name::$item),*];
        }

        impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg> for $name {
            fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
                match <i32 as diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::pg::Pg>>::from_sql(bytes)? {
                    $($int => Ok(Self::$item),)*
                    n => Err(format!("unknown value for {}: {}", stringify!($name), n).into()),
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
