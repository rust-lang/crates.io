use crate::util::TestApp;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;

#[tokio::test(flavor = "multi_thread")]
async fn skips_when_crate_deleted() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let job = jobs::SendPublishNotificationsJob::new(-1);

    job.enqueue(&mut conn).await?;
    app.run_pending_background_jobs().await;

    Ok(())
}
