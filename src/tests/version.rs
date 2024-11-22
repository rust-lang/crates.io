use crate::models::Version;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::TestApp;

#[tokio::test(flavor = "multi_thread")]
async fn record_rerendered_readme_time() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let c = CrateBuilder::new("foo_authors", user.id)
        .expect_build(&mut conn)
        .await;
    let version = VersionBuilder::new("1.0.0")
        .expect_build(c.id, user.id, &mut conn)
        .await;

    let mut conn = app.db_conn().await;
    Version::record_readme_rendering(version.id, &mut conn).await?;
    Version::record_readme_rendering(version.id, &mut conn).await?;

    Ok(())
}
