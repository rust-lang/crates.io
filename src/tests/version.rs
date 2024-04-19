use crate::builders::{CrateBuilder, VersionBuilder};
use crate::TestApp;
use crates_io::models::Version;

#[tokio::test(flavor = "multi_thread")]
async fn record_rerendered_readme_time() {
    let (app, _, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c = CrateBuilder::new("foo_authors", user.id).expect_build(conn);
        let version = VersionBuilder::new("1.0.0").expect_build(c.id, user.id, conn);

        Version::record_readme_rendering(version.id, conn).unwrap();
        Version::record_readme_rendering(version.id, conn).unwrap();
    });
}
