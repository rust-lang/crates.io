use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text};
use diesel_async::RunQueryDsl;
use std::fmt::Debug;

const SEMVER_KEY_ORD_QUERY: &str = r#"
create or replace function semver_key_ord(num text) returns bytea as $$
declare
    major text := substring(num from '\d+');
    minor text := substring(num from '\d+\.(\d+)');
    result bytea;
begin
    if major is null then
        return null;
    end if;

    if major = '0' then
        result := semver_ord_num(major) || semver_ord_num(minor);
    else
        result := semver_ord_num(major);
    end if;

    return result;
end;
$$ language 'plpgsql' immutable parallel safe strict;"#;

#[tokio::test]
async fn test_semver_key_ord_bytea_output() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    diesel::sql_query(SEMVER_KEY_ORD_QUERY)
        .execute(&mut conn)
        .await
        .unwrap();

    let mut check = async |num| {
        let query = format!("select encode(semver_key_ord('{num}'), 'hex') as output");

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

    insta::assert_snapshot!(check("0.0.0").await, @"01300130");
    insta::assert_snapshot!(check("1.0.0-alpha.1").await, @"0131");

    // Build metadata is discarded; these match `1.0.0` and `1.2.3-alpha.1` respectively.
    insta::assert_snapshot!(check("1.0.0+build.1").await, @"0131");
    insta::assert_snapshot!(check("1.2.3-alpha.1+exp.sha.5114f85").await, @"0131");

    // see https://crates.io/crates/cursed-trying-to-break-cargo/1.0.0-0.HDTV-BluRay.1020p.YTSUB.L33TRip.mkv – thanks @Gankra!
    insta::assert_snapshot!(check("1.0.0-0.HDTV-BluRay.1020p.YTSUB.L33TRip.mkv").await, @"0131");

    // Invalid version string
    insta::assert_snapshot!(check("foo").await, @"");

    // Version string with a lot of prerelease identifiers
    insta::assert_snapshot!(check("1.2.3-1.2.3.4.5.6.7.8.9.10.11.12.13.14.15.16.17.end").await, @"0131");
}

#[tokio::test]
async fn test_semver_key_ord_order() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    diesel::sql_query(SEMVER_KEY_ORD_QUERY)
        .execute(&mut conn)
        .await
        .unwrap();

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
        let rev_order = if order == "asc" { "desc" } else { "asc" };
        let query = format!(
            r#"
            with nums as (
                select unnest(array[
                    '100.1.1',
                    '100.1.0',
                    '1.4.0-beta.1',
                    '1.3.5',
                    '1.2.5',
                    '1.1.5',
                    '0.4.0-rc.1',
                    '0.3.23',
                    '0.3.22',
                    '0.3.21-pre.0',
                    '0.3.20',
                    '0.3.3',
                    '0.3.2',
                    '0.3.1',
                    '0.3.0',
                    '0.2.1',
                    '0.2.0',
                    '0.1.2',
                    '0.1.1'
                ]) as num
            ), d as (
                select num
                , encode(semver_key_ord(num), 'escape') as encoded
                , semver_key_ord(num)
                , semver_ord_v2(num)
                from nums
            )
            select num
            from d
            order by semver_key_ord {order}
            , get_byte(semver_ord_v2, octet_length(semver_ord_v2) - 1) <> '3' {rev_order}
            , semver_ord_v2 {order}
            "#
        );

        diesel::sql_query(query)
            .load::<Row>(&mut conn)
            .await
            .unwrap()
    };

    insta::assert_debug_snapshot!(check("asc").await, @r"
    [
        0.1.1,
        0.1.2,
        0.2.0,
        0.2.1,
        0.3.21-pre.0,
        0.3.0,
        0.3.1,
        0.3.2,
        0.3.3,
        0.3.20,
        0.3.22,
        0.3.23,
        0.4.0-rc.1,
        1.4.0-beta.1,
        1.1.5,
        1.2.5,
        1.3.5,
        100.1.0,
        100.1.1,
    ]
    ");

    insta::assert_debug_snapshot!(check("desc").await, @r"
    [
        100.1.1,
        100.1.0,
        1.3.5,
        1.2.5,
        1.1.5,
        1.4.0-beta.1,
        0.4.0-rc.1,
        0.3.23,
        0.3.22,
        0.3.20,
        0.3.3,
        0.3.2,
        0.3.1,
        0.3.0,
        0.3.21-pre.0,
        0.2.1,
        0.2.0,
        0.1.2,
        0.1.1,
    ]
    ");
}

#[tokio::test]
async fn test_semver_key_ord_highest() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    diesel::sql_query(SEMVER_KEY_ORD_QUERY)
        .execute(&mut conn)
        .await
        .unwrap();

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

    let mut check = async || {
        let query = r#"
            with nums as (
                select unnest(array[
                    '100.1.1',
                    '100.1.0',
                    '1.4.0-beta.1',
                    '1.3.5',
                    '1.2.5',
                    '1.1.5',
                    '0.4.0-rc.1',
                    '0.3.23',
                    '0.3.22',
                    '0.3.21-pre.0',
                    '0.3.20',
                    '0.3.3',
                    '0.3.2',
                    '0.3.1',
                    '0.3.0',
                    '0.2.1',
                    '0.2.0',
                    '0.1.2',
                    '0.1.1'
                ]) as num
            ), d as (
                select num
                , encode(semver_key_ord(num), 'escape') as encoded
                , semver_key_ord(num)
                , semver_ord_v2(num)
                from nums
            )
            select distinct on (semver_key_ord) num
            from d
            order by semver_key_ord desc
            , get_byte(semver_ord_v2, octet_length(semver_ord_v2) - 1) <> '3'
            , semver_ord_v2 desc
            "#;

        diesel::sql_query(query)
            .load::<Row>(&mut conn)
            .await
            .unwrap()
    };

    // "100": { "highest": "100.1.1" },
    // "1": { "highest": "1.3.5" },
    // "0.4": { "highest": "0.4.0-rc.1" },
    // "0.3": { "highest": "0.3.23" },
    // "0.2": { "highest": "0.2.1" },
    // "0.1": { "highest": "0.1.2" }
    insta::assert_debug_snapshot!(check().await, @r"
    [
        100.1.1,
        1.3.5,
        0.4.0-rc.1,
        0.3.23,
        0.2.1,
        0.1.2,
    ]
    ");
}
