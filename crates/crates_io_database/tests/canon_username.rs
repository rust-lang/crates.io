//! Tests for the `canon_username()` SQL function, which normalizes usernames so
//! that case and `-`/`_` differences collide. The function is evaluated
//! directly against the database rather than through the reserved-username
//! trigger that relies on it.

use crates_io_database::fns::canon_username;
use crates_io_test_db::TestDatabase;
use diesel_async::RunQueryDsl;

#[tokio::test]
async fn canon_username_normalizes_case_and_separators() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let cases = [
        ("foo", "foo"),
        ("Foo", "foo"),
        ("FOO", "foo"),
        ("foo-bar", "foo_bar"),
        ("foo_bar", "foo_bar"),
        ("Foo-Bar", "foo_bar"),
        ("FOO-BAR", "foo_bar"),
        ("a-b-c", "a_b_c"),
        ("-foo-", "_foo_"),
        ("", ""),
    ];

    for (input, expected) in cases {
        let result: String = diesel::select(canon_username(input))
            .get_result(&mut conn)
            .await
            .unwrap();

        assert_eq!(result, expected, "canon_username({input:?})");
    }
}
