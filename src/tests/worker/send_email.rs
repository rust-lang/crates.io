use crate::util::TestApp;
use crates_io::email::EmailMessage;
use crates_io::worker::jobs::SendEmail;
use crates_io_worker::BackgroundJob;
use insta::assert_snapshot;

/// Queued delivery preserves the prepared message without loading its template.
#[tokio::test(flavor = "multi_thread")]
async fn sends_rendered_email() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let conn = app.db_conn().await;
    let recipient = "recipient@example.com".parse()?;
    let job = SendEmail::new(recipient, prepared_email());
    job.enqueue(&conn).await?;
    assert!(app.emails().await.is_empty());

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.emails_snapshot().await);

    Ok(())
}

/// Builds a message whose template does not exist at delivery time.
fn prepared_email() -> EmailMessage {
    EmailMessage {
        template_name: "removed_template".into(),
        subject: "Prepared subject".into(),
        body_text: "Prepared plain text".into(),
        body_html: "<p>Prepared HTML</p>".into(),
    }
}
