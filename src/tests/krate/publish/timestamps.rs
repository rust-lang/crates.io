#[tokio::test(flavor = "multi_thread")]
async fn uploading_new_version_touches_crate() {
    use crate::schema::crates;
    use crate::tests::builders::PublishBuilder;
    use crate::tests::util::{RequestHelper, TestApp};
    use crate::tests::CrateResponse;
    use diesel::dsl::*;
    use diesel::{ExpressionMethods, RunQueryDsl};

    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at", "1.0.0");
    user.publish_crate(crate_to_publish).await.good();

    app.db(|conn| {
        diesel::update(crates::table)
            .set(crates::updated_at.eq(crates::updated_at - 1.hour()))
            .execute(conn)
            .unwrap();
    });

    let json: CrateResponse = user.show_crate("foo_versions_updated_at").await;
    let updated_at_before = json.krate.updated_at;

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at", "2.0.0");
    user.publish_crate(crate_to_publish).await.good();

    let json: CrateResponse = user.show_crate("foo_versions_updated_at").await;
    let updated_at_after = json.krate.updated_at;

    assert_ne!(updated_at_before, updated_at_after);
}
