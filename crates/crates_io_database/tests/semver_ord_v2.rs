use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text};
use diesel_async::RunQueryDsl;
use std::fmt::Debug;

#[tokio::test]
async fn test_bytea_output() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let mut check = async |num| {
        let query = format!("select encode(semver_ord_v2('{num}'), 'hex') as output");

        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Nullable<Text>)]
            output: Option<String>,
        }

        diesel::sql_query(query)
            .get_result::<Row>(&mut conn)
            .await
            .unwrap()
            .output
            .unwrap_or_default()
    };

    insta::assert_snapshot!(check("0.0.0").await, @"01300130013003");
    insta::assert_snapshot!(check("1.0.0-alpha.1").await, @"01310130013002616c70686101013100");

    // Build metadata is discarded; these match `1.0.0` and `1.2.3-alpha.1` respectively.
    insta::assert_snapshot!(check("1.0.0+build.1").await, @"01310130013003");
    insta::assert_snapshot!(check("1.2.3-alpha.1+exp.sha.5114f85").await, @"01310132013302616c70686101013100");

    // see https://crates.io/crates/cursed-trying-to-break-cargo/1.0.0-0.HDTV-BluRay.1020p.YTSUB.L33TRip.mkv – thanks @Gankra!
    insta::assert_snapshot!(check("1.0.0-0.HDTV-BluRay.1020p.YTSUB.L33TRip.mkv").await, @"01310130013001013002484454562d426c75526179023130323070025954535542024c333354526970026d6b7600");

    // Invalid version string
    insta::assert_snapshot!(check("foo").await, @"");

    // Version string with a lot of prerelease identifiers (no upper bound in v2)
    insta::assert_snapshot!(check("1.2.3-1.2.3.4.5.6.7.8.9.10.11.12.13.14.15.16.17.end").await, @"013101320133010131010132010133010134010135010136010137010138010139010231300102313101023132010231330102313401023135010231360102313702656e6400");
}

/// This test checks that the `semver_ord_v2` function orders versions correctly.
///
/// The test data is a list of versions in a random order. The versions are then
/// ordered by the `semver_ord_v2` function and the result is compared to the
/// expected order (see <https://semver.org/#spec-item-11>).
///
/// The test data was imported from <https://github.com/dtolnay/semver/blob/1.0.26/tests/test_version.rs#L223-L242>.
#[tokio::test]
async fn test_spec_order() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

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

    let mut check = async |order| {
        let query = format!(
            r#"
            with nums as (
                select unnest(array[
                    '1.0.0-beta',
                    '1.0.0-alpha',
                    '1.0.0-rc.1',
                    '1.0.0',
                    '1.0.0-beta.2',
                    '1.0.0-alpha.1',
                    '1.0.0-alpha.beta',
                    '1.0.0-beta.11',
                    '1.0.0-B'
                ]) as num
            )
            select num
            from nums
            order by semver_ord_v2(num) {order};
            "#
        );

        diesel::sql_query(query)
            .load::<Row>(&mut conn)
            .await
            .unwrap()
    };

    insta::assert_debug_snapshot!(check("asc").await, @r"
    [
        1.0.0-B,
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

    insta::assert_debug_snapshot!(check("desc").await, @r"
    [
        1.0.0,
        1.0.0-rc.1,
        1.0.0-beta.11,
        1.0.0-beta.2,
        1.0.0-beta,
        1.0.0-alpha.beta,
        1.0.0-alpha.1,
        1.0.0-alpha,
        1.0.0-B,
    ]
    ");
}
