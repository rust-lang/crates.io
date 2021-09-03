use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::{select, sql_function};

fn pg_connection() -> PgConnection {
    let database_url =
        dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
    let conn = PgConnection::establish(&database_url).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}

sql_function!(fn to_semver_no_prerelease(x: Text) -> Nullable<Record<(Numeric, Numeric, Numeric)>>);

#[test]
fn to_semver_no_prerelease_works() {
    let conn = pg_connection();

    #[track_caller]
    fn test(conn: &PgConnection, text: &str, expected: Option<(i32, i32, i32)>) {
        let query = select(to_semver_no_prerelease(text));
        let result = query
            .get_result::<Option<(BigDecimal, BigDecimal, BigDecimal)>>(conn)
            .unwrap();

        let expected = expected.map(|it| (it.0.into(), it.1.into(), it.2.into()));
        assert_eq!(result, expected);
    }

    test(&conn, "0.0.0", Some((0, 0, 0)));
    test(&conn, "1.2.4", Some((1, 2, 4)));
    test(&conn, "1.2.4+metadata", Some((1, 2, 4)));
    test(&conn, "1.2.4-beta.3", None);
}
