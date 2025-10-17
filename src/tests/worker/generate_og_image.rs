use crate::tests::builders::CrateBuilder;
use crate::tests::util::TestApp;
use claims::{assert_err, assert_ok};
use crates_io_env_vars::var;
use crates_io_worker::BackgroundJob;
use insta::assert_binary_snapshot;
use std::process::Command;
use tracing::warn;

fn is_ci() -> bool {
    var("CI").unwrap().is_some()
}

fn typst_available() -> bool {
    Command::new("typst").arg("--version").spawn().is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_generate_og_image_job() {
    let (app, _, user) = TestApp::full().with_og_image_generator().with_user().await;

    if !is_ci() && !typst_available() {
        warn!("Skipping OG image generation test because 'typst' is not available");
        return;
    }

    let mut conn = app.db_conn().await;

    // Create a test crate with keywords using CrateBuilder
    CrateBuilder::new("test-crate", user.as_model().id)
        .description("A test crate for OG image generation")
        .keyword("testing")
        .keyword("rust")
        .expect_build(&mut conn)
        .await;

    // Create and enqueue the job
    let job = crate::worker::jobs::GenerateOgImage::new("test-crate".to_string());
    job.enqueue(&mut conn).await.unwrap();

    // Run the background job
    app.run_pending_background_jobs().await;

    // Verify the OG image was uploaded to storage
    let storage = app.as_inner().storage.as_inner();
    let og_image_path = "og-images/test-crate.png";

    // Try to download the image to verify it exists
    let download_result = storage.get(&og_image_path.into()).await;
    let result = assert_ok!(
        download_result,
        "OG image should be uploaded to storage at: {og_image_path}"
    );

    // Verify it's a non-empty file
    let image_bytes = result.bytes().await.unwrap().to_vec();
    assert!(!image_bytes.is_empty(), "OG image should not be empty");

    // Verify it starts with PNG magic bytes
    assert_eq!(
        &image_bytes[0..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "Uploaded file should be a valid PNG"
    );

    assert_binary_snapshot!("og-image.png", image_bytes);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_generate_og_image_job_nonexistent_crate() {
    let (app, _, _) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create and enqueue the job for a non-existent crate
    let job = crate::worker::jobs::GenerateOgImage::new("nonexistent-crate".to_string());
    job.enqueue(&mut conn).await.unwrap();

    // Run the background job - should complete without error
    app.run_pending_background_jobs().await;

    // Verify no OG image was uploaded
    let storage = app.as_inner().storage.as_inner();
    let og_image_path = "og-images/nonexistent-crate.png";
    let download_result = storage.get(&og_image_path.into()).await;
    assert_err!(
        download_result,
        "No OG image should be uploaded for nonexistent crate"
    );
}
