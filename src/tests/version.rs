use crate::models::Version;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::TestApp;

#[tokio::test(flavor = "multi_thread")]
async fn record_rerendered_readme_time() {
    let (app, _, user) = TestApp::init().with_user();
    let mut conn = app.db_conn();
    let user = user.as_model();

    let c = CrateBuilder::new("foo_authors", user.id).expect_build(&mut conn);
    let version = VersionBuilder::new("1.0.0").expect_build(c.id, user.id, &mut conn);

    let mut conn = app.async_db_conn().await;
    Version::record_readme_rendering(version.id, &mut conn)
        .await
        .unwrap();
    Version::record_readme_rendering(version.id, &mut conn)
        .await
        .unwrap();
}
