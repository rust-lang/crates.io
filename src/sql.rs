use diesel::sql_types::{Date, Double, Integer, Interval, SingleValue, Text, Timestamp};

mod semver;

pub use semver::SemverVersion;

define_sql_function!(#[aggregate] fn array_agg<T: SingleValue>(x: T) -> Array<T>);
define_sql_function!(fn canon_crate_name(x: Text) -> Text);
define_sql_function!(fn to_char(a: Date, b: Text) -> Text);
define_sql_function!(fn lower(x: Text) -> Text);
define_sql_function!(fn date_part(x: Text, y: Timestamp) -> Double);
define_sql_function! {
    #[sql_name = "date_part"]
    fn interval_part(x: Text, y: Interval) -> Double;
}
define_sql_function!(fn floor(x: Double) -> Integer);
define_sql_function!(fn greatest<T: SingleValue>(x: T, y: T) -> T);
define_sql_function!(fn least<T: SingleValue>(x: T, y: T) -> T);
define_sql_function!(fn split_part(string: Text, delimiter: Text, n: Integer) -> Text);

macro_rules! pg_enum {
    (
        $vis:vis enum $name:ident {
            $($item:ident = $int:expr,)*
        }
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, FromSqlRow, AsExpression)]
        #[diesel(sql_type = diesel::sql_types::Integer)]
        #[serde(rename_all = "snake_case")]
        #[repr(i32)]
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

pub(crate) use pg_enum;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sql_types::semver::Triple;
    use crate::schema::sql_types::SemverTriple;
    use crate::test_util::test_db_connection;
    use diesel::prelude::*;
    use diesel::select;

    define_sql_function!(fn to_semver_no_prerelease(x: Text) -> Nullable<SemverTriple>);

    #[test]
    fn to_semver_no_prerelease_works() {
        let (_test_db, mut conn) = test_db_connection();

        #[track_caller]
        fn test(conn: &mut PgConnection, text: &str, expected: Option<(u64, u64, u64)>) {
            let query = select(to_semver_no_prerelease(text));
            let result = query
                .get_result::<Option<Triple>>(conn)
                .unwrap()
                .map(|triple| (triple.major, triple.minor, triple.teeny));

            assert_eq!(result, expected);
        }

        test(&mut conn, "0.0.0", Some((0, 0, 0)));
        test(&mut conn, "1.2.4", Some((1, 2, 4)));
        test(&mut conn, "1.2.4+metadata", Some((1, 2, 4)));
        test(&mut conn, "1.2.4-beta.3", None);
        // see https://github.com/rust-lang/crates.io/issues/3882
        test(&mut conn, "0.4.45+curl-7.78.0", Some((0, 4, 45)));
        test(&mut conn, "0.1.4-preview+4.3.2", None);
    }
}
