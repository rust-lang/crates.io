use claims::{assert_none, assert_some_eq};
use crates_io_database::fns::to_bigint;
use crates_io_test_db::TestDatabase;
use diesel::dsl::sql;
use diesel::result::Error;
use diesel::sql_types::{Nullable, Numeric};
use diesel_async::RunQueryDsl;

/// Checks fractional rounding, including ties away from zero.
#[tokio::test]
async fn rounds_fractional_values() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let cases = [
        ("0.0", 0),
        ("1.4", 1),
        ("1.5", 2),
        ("1.6", 2),
        ("2.5", 3),
        ("-1.4", -1),
        ("-1.5", -2),
        ("-1.6", -2),
        ("-2.5", -3),
    ];

    for (input, expected) in cases {
        let result = diesel::select(to_bigint(sql::<Nullable<Numeric>>(input)))
            .get_result::<Option<i64>>(&mut conn)
            .await
            .unwrap();

        assert_some_eq!(result, expected, "to_bigint({input})");
    }
}

/// Checks null propagation.
#[tokio::test]
async fn propagates_null() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let input = sql::<Nullable<Numeric>>("NULL::numeric");
    let result = diesel::select(to_bigint(input))
        .get_result::<Option<i64>>(&mut conn)
        .await
        .unwrap();

    assert_none!(result);
}

/// Checks values that round to the integer limits.
#[tokio::test]
async fn handles_integer_limits() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let cases = [
        ("9223372036854775807.0", i64::MAX),
        ("9223372036854775807.4", i64::MAX),
        ("-9223372036854775808.0", i64::MIN),
        ("-9223372036854775808.4", i64::MIN),
    ];

    for (input, expected) in cases {
        let result = diesel::select(to_bigint(sql::<Nullable<Numeric>>(input)))
            .get_result::<Option<i64>>(&mut conn)
            .await
            .unwrap();

        assert_some_eq!(result, expected, "to_bigint({input})");
    }
}

/// Checks overflow in both directions, including values rounded out of range.
#[tokio::test]
async fn rejects_overflow() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let cases = [
        "9223372036854775808.0",
        "9223372036854775807.5",
        "-9223372036854775809.0",
        "-9223372036854775808.5",
    ];

    for input in cases {
        let error = diesel::select(to_bigint(sql::<Nullable<Numeric>>(input)))
            .get_result::<Option<i64>>(&mut conn)
            .await
            .unwrap_err();

        std::assert_matches!(error, Error::DatabaseError(..));
        let message = error.to_string();
        assert_eq!(message, "bigint out of range", "to_bigint({input})");
    }
}
