use crate::index::index_metadata;
use crate::schema::users;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::insta::assert_json_snapshot;
use chrono::{Days, Utc};
use crates_io_test_db::TestDatabase;
use diesel::{ExpressionMethods, RunQueryDsl};

#[test]
fn test_index_metadata() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.connect();

    let user_id = diesel::insert_into(users::table)
        .values((
            users::name.eq("user1"),
            users::gh_login.eq("user1"),
            users::gh_id.eq(42),
            users::gh_access_token.eq("some random token"),
        ))
        .returning(users::id)
        .get_result::<i32>(&mut conn)
        .unwrap();

    let created_at_1 = Utc::now()
        .checked_sub_days(Days::new(14))
        .unwrap()
        .naive_utc();

    let created_at_2 = Utc::now()
        .checked_sub_days(Days::new(7))
        .unwrap()
        .naive_utc();

    let fooo = CrateBuilder::new("foo", user_id)
        .version(VersionBuilder::new("0.1.0"))
        .expect_build(&mut conn);

    let metadata = index_metadata(&fooo, &mut conn).unwrap();
    assert_json_snapshot!(metadata);

    let bar = CrateBuilder::new("bar", user_id)
        .version(
            VersionBuilder::new("1.0.0-beta.1")
                .created_at(created_at_1)
                .yanked(true),
        )
        .version(VersionBuilder::new("1.0.0").created_at(created_at_1))
        .version(
            VersionBuilder::new("2.0.0")
                .created_at(created_at_2)
                .dependency(&fooo, None),
        )
        .version(VersionBuilder::new("1.0.1").checksum("0123456789abcdef"))
        .expect_build(&mut conn);

    let metadata = index_metadata(&bar, &mut conn).unwrap();
    assert_json_snapshot!(metadata);
}
