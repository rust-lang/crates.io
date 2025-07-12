use crate::tests::util::TestApp;
use crate::worker::jobs;
use crates_io_worker::BackgroundJob;

#[tokio::test(flavor = "multi_thread")]
async fn skips_when_crate_deleted() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let job =
        jobs::RenderAndUploadReadme::new(-1, "deleted".to_string(), ".".to_string(), None, None);

    job.enqueue(&mut conn).await?;
    app.run_pending_background_jobs().await;

    Ok(())
}
