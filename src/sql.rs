use diesel::sql_types::{Date, Double, Integer, Interval, SingleValue, Text, Timestamp};

sql_function!(#[aggregate] fn array_agg<T: SingleValue>(x: T) -> Array<T>);
sql_function!(fn canon_crate_name(x: Text) -> Text);
sql_function!(fn to_char(a: Date, b: Text) -> Text);
sql_function!(fn lower(x: Text) -> Text);
sql_function!(fn date_part(x: Text, y: Timestamp) -> Double);
sql_function! {
    #[sql_name = "date_part"]
    fn interval_part(x: Text, y: Interval) -> Double;
}
sql_function!(fn floor(x: Double) -> Integer);
sql_function!(fn greatest<T: SingleValue>(x: T, y: T) -> T);
sql_function!(fn least<T: SingleValue>(x: T, y: T) -> T);
sql_function!(fn split_part(string: Text, delimiter: Text, n: Integer) -> Text);

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
