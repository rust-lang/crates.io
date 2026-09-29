use crate::util::TestApp;
use claims::assert_ok;
use crates_io::schema::background_jobs;
use crates_io::worker::jobs;
use crates_io_github::{GitHubAuth, MockGitHubClient};
use crates_io_worker::BackgroundJob;
use diesel_async::RunQueryDsl;
use secrecy::ExposeSecret;

const INDEX_URL: &str = "https://github.com/rust-lang/crates.io-index.git";

/// Deletes only the archived snapshot ref from the source index repository.
#[tokio::test(flavor = "multi_thread")]
async fn delete_archived_index_branch() {
    let mut github = MockGitHubClient::new();
    github
        .expect_delete_ref()
        .withf(|owner, repo, ref_name, auth| {
            owner == "rust-lang"
                && repo == "crates.io-index"
                && ref_name == "refs/heads/snapshot-test"
                && matches!(auth, GitHubAuth::Bearer { token } if token.expose_secret() == "test-token")
        })
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let (app, _) = TestApp::init()
        .with_github(github)
        .with_index_location(INDEX_URL.parse().unwrap())
        .with_job_runner()
        .empty()
        .await;

    let conn = app.db_conn().await;

    let job = jobs::DeleteArchivedIndexBranch::new("snapshot-test");
    assert_ok!(job.enqueue(&conn).await);
    app.run_pending_background_jobs().await;
}

/// Rejects an unrelated branch before making a GitHub request.
#[tokio::test(flavor = "multi_thread")]
async fn rejects_non_snapshot_branch() {
    let mut github = MockGitHubClient::new();
    github.expect_delete_ref().returning(|_, _, _, _| Ok(()));

    let (app, _) = TestApp::init()
        .with_github(github)
        .with_index_location(INDEX_URL.parse().unwrap())
        .with_job_runner()
        .empty()
        .await;

    let mut conn = app.db_conn().await;

    let job = jobs::DeleteArchivedIndexBranch::new("master");
    assert_ok!(job.enqueue(&conn).await);

    let error = app.try_run_pending_background_jobs().await.unwrap_err();
    assert_eq!(error.to_string(), "1 jobs failed");

    diesel::delete(background_jobs::table)
        .execute(&mut conn)
        .await
        .unwrap();
}
