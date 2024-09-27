use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::insta::assert_json_snapshot;
use crate::tests::TestApp;
use chrono::{Days, Utc};

#[test]
fn index_metadata() {
    let (app, _, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let created_at_1 = Utc::now()
            .checked_sub_days(Days::new(14))
            .unwrap()
            .naive_utc();

        let created_at_2 = Utc::now()
            .checked_sub_days(Days::new(7))
            .unwrap()
            .naive_utc();

        let fooo = CrateBuilder::new("foo", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .expect_build(conn);

        let metadata = fooo.index_metadata(conn).unwrap();
        assert_json_snapshot!(metadata);

        let bar = CrateBuilder::new("bar", user.id)
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
            .expect_build(conn);

        let metadata = bar.index_metadata(conn).unwrap();
        assert_json_snapshot!(metadata);
    });
}
