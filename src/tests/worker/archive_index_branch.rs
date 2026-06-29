use crate::util::TestApp;
use claims::assert_ok;
use crates_io::schema::background_jobs;
use crates_io::worker::jobs;
use crates_io_index::testing::UpstreamIndex;
use crates_io_worker::BackgroundJob;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;

const SNAPSHOT_BRANCH: &str = "snapshot-test";

/// Seed a `snapshot-test` branch on the primary upstream. Mirrors the
/// post-squash shape: a parentless commit holding pre-squash index entries,
/// sharing no common ancestor with `master`.
fn seed_snapshot_branch(upstream: &UpstreamIndex) {
    upstream
        .create_orphan_branch(SNAPSHOT_BRANCH, "1/a", "a\n")
        .unwrap();
}

/// `ArchiveIndexBranch` should mirror the snapshot branch to the configured
/// archive repository.
#[tokio::test(flavor = "multi_thread")]
async fn archive_index_branch() {
    let archive = UpstreamIndex::new().unwrap();
    let archive_url = archive.url();

    let (app, _) = TestApp::full()
        .with_git_index()
        .with_config(|c| c.index_archive_url = Some(archive_url))
        .empty()
        .await;

    let conn = app.db_conn().await;
    seed_snapshot_branch(app.upstream_index());
    let expected_oid = app.upstream_index().branch_oid(SNAPSHOT_BRANCH).unwrap();

    let job = jobs::ArchiveIndexBranch::new(SNAPSHOT_BRANCH);
    assert_ok!(job.enqueue(&conn).await);
    app.run_pending_background_jobs().await;

    assert_eq!(archive.branch_oid(SNAPSHOT_BRANCH).unwrap(), expected_oid);
}

/// With no `index_archive_url` configured, the job should succeed as a no-op
/// without touching any repositories.
#[tokio::test(flavor = "multi_thread")]
async fn archive_index_branch_without_url_configured() {
    let (app, _) = TestApp::full().with_git_index().empty().await;
    let conn = app.db_conn().await;

    // Seed a branch on origin so the test isn't trivially vacuous: the job
    // should return before ever attempting a fetch, regardless.
    seed_snapshot_branch(app.upstream_index());

    let job = jobs::ArchiveIndexBranch::new(SNAPSHOT_BRANCH);
    assert_ok!(job.enqueue(&conn).await);
    app.run_pending_background_jobs().await;
}

/// With an archive URL configured but no index sync GitHub App wired into the
/// environment, the job should fail loudly rather than push without
/// authentication.
#[tokio::test(flavor = "multi_thread")]
async fn archive_index_branch_without_index_sync_github_app() {
    let archive = UpstreamIndex::new().unwrap();
    let archive_url = archive.url();

    let (app, _) = TestApp::full()
        .with_git_index()
        .with_config(|c| c.index_archive_url = Some(archive_url))
        .with_index_sync_github_app(None)
        .empty()
        .await;

    let mut conn = app.db_conn().await;
    seed_snapshot_branch(app.upstream_index());

    let job = jobs::ArchiveIndexBranch::new(SNAPSHOT_BRANCH);
    assert_ok!(job.enqueue(&conn).await);
    assert_snapshot!(app.try_run_pending_background_jobs().await.unwrap_err(), @"1 jobs failed");

    diesel::delete(background_jobs::table)
        .execute(&mut conn)
        .await
        .unwrap();
}

/// If the requested branch does not exist on origin, the job should fail so
/// that an operator typo or a stale enqueue produces loud feedback rather
/// than a silent success.
#[tokio::test(flavor = "multi_thread")]
async fn archive_index_branch_missing_branch() {
    let archive = UpstreamIndex::new().unwrap();
    let archive_url = archive.url();

    let (app, _) = TestApp::full()
        .with_git_index()
        .with_config(|c| c.index_archive_url = Some(archive_url))
        .empty()
        .await;

    let mut conn = app.db_conn().await;

    let job = jobs::ArchiveIndexBranch::new("does-not-exist");
    assert_ok!(job.enqueue(&conn).await);
    assert_snapshot!(app.try_run_pending_background_jobs().await.unwrap_err(), @"1 jobs failed");

    // The archive repo must not have gained a matching branch; we expect a
    // `NotFound` error back with a stable message.
    {
        let archive_repo = archive.repository.lock().unwrap();
        let err = archive_repo
            .find_reference("refs/heads/does-not-exist")
            .err()
            .unwrap();

        assert_snapshot!(err, @"reference 'refs/heads/does-not-exist' not found; class=Reference (4); code=NotFound (-3)");
    }

    // Drain the failed job so the `TestAppInner::drop` post-condition that
    // asserts an empty queue is satisfied.
    diesel::delete(background_jobs::table)
        .execute(&mut conn)
        .await
        .unwrap();
}
