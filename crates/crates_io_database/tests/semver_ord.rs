use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;
use std::fmt::Debug;

/// This test checks that the `semver_ord` function orders versions correctly.
///
/// The test data is a list of versions in a random order. The versions are then
/// ordered by the `semver_ord` function and the result is compared to the
/// expected order (see <https://semver.org/#spec-item-11>).
///
/// The test data was imported from <https://github.com/dtolnay/semver/blob/1.0.26/tests/test_version.rs#L223-L242>.
#[tokio::test]
async fn test_spec_order() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let query = r#"
    with nums as (
        select unnest(array[
            '1.0.0-beta',
            '1.0.0-alpha',
            '1.0.0-rc.1',
            '1.0.0',
            '1.0.0-beta.2',
            '1.0.0-alpha.1',
            '1.0.0-alpha.beta',
            '1.0.0-beta.11'
        ]) as num
    )
    select num
    from nums
    order by semver_ord(num);
    "#;

    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        num: String,
    }

    impl Debug for Row {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.num)
        }
    }

    let nums = diesel::sql_query(query)
        .load::<Row>(&mut conn)
        .await
        .unwrap();

    insta::assert_debug_snapshot!(nums, @r"
    [
        1.0.0-alpha,
        1.0.0-alpha.1,
        1.0.0-alpha.beta,
        1.0.0-beta,
        1.0.0-beta.2,
        1.0.0-beta.11,
        1.0.0-rc.1,
        1.0.0,
    ]
    ");
}
