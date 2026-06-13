use crate::util::TestApp;
use claims::assert_ok;
use crates_io::schema::background_jobs;
use crates_io::worker::jobs;
use crates_io_github::{GitCommit, GitObject, GitRef, MockGitHubClient};
use crates_io_worker::BackgroundJob;
use diesel_async::RunQueryDsl;
use url::Url;

const OWNER: &str = "rust-lang";
const REPO: &str = "crates.io-index";
const MASTER_REF: &str = "refs/heads/master";
const ORIGINAL_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE_SHA: &str = "ffffffffffffffffffffffffffffffffffffffff";
const NEW_SHA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn index_url() -> Url {
    format!("https://github.com/{OWNER}/{REPO}.git")
        .parse()
        .unwrap()
}

fn master_ref(sha: &str) -> GitRef {
    GitRef {
        ref_name: MASTER_REF.into(),
        object: GitObject { sha: sha.into() },
    }
}

/// Queue a one-shot `get_ref(refs/heads/master)` that returns `sha`.
fn expect_get_master(mock: &mut MockGitHubClient, sha: &'static str) {
    mock.expect_get_ref()
        .withf(|owner, repo, ref_name, _| owner == OWNER && repo == REPO && ref_name == MASTER_REF)
        .times(1)
        .returning(move |_, _, _, _| Ok(master_ref(sha)));
}

/// Queue a `get_commit(commit_sha)` that returns a commit pointing at `tree_sha`.
fn expect_get_commit(
    mock: &mut MockGitHubClient,
    commit_sha: &'static str,
    tree_sha: &'static str,
) {
    mock.expect_get_commit()
        .withf(move |owner, repo, sha, _| owner == OWNER && repo == REPO && sha == commit_sha)
        .times(1)
        .returning(move |_, _, _, _| {
            Ok(GitCommit {
                sha: commit_sha.into(),
                tree: GitObject {
                    sha: tree_sha.into(),
                },
            })
        });
}

/// Queue a parentless `create_commit(tree=tree_sha)` that returns `new_sha`.
fn expect_create_commit(
    mock: &mut MockGitHubClient,
    tree_sha: &'static str,
    new_sha: &'static str,
) {
    mock.expect_create_commit()
        .withf(move |owner, repo, input, _| {
            owner == OWNER
                && repo == REPO
                && input.tree == tree_sha
                && input.parents.is_empty()
                && input.message.starts_with("Collapse index into one commit")
        })
        .times(1)
        .returning(move |_, _, _, _| {
            Ok(GitCommit {
                sha: new_sha.into(),
                tree: GitObject {
                    sha: tree_sha.into(),
                },
            })
        });
}

/// Queue a `create_ref(refs/heads/snapshot-*, original_sha)`.
fn expect_create_snapshot_ref(mock: &mut MockGitHubClient, original_sha: &'static str) {
    mock.expect_create_ref()
        .withf(move |owner, repo, ref_name, sha, _| {
            owner == OWNER
                && repo == REPO
                && ref_name.starts_with("refs/heads/snapshot-")
                && sha == original_sha
        })
        .times(1)
        .returning(|_, _, ref_name, sha, _| {
            Ok(GitRef {
                ref_name: ref_name.to_string(),
                object: GitObject {
                    sha: sha.to_string(),
                },
            })
        });
}

/// Queue a forced `update_ref(refs/heads/master, new_sha, force=true)`.
fn expect_update_master(mock: &mut MockGitHubClient, new_sha: &'static str) {
    mock.expect_update_ref()
        .withf(move |owner, repo, ref_name, sha, force, _| {
            owner == OWNER && repo == REPO && ref_name == MASTER_REF && sha == new_sha && *force
        })
        .times(1)
        .returning(|_, _, ref_name, sha, _, _| {
            Ok(GitRef {
                ref_name: ref_name.to_string(),
                object: GitObject {
                    sha: sha.to_string(),
                },
            })
        });
}

/// `SquashIndex` should drive the squash entirely via the GitHub REST
/// API: read master, read its tree, create a parentless commit on the same
/// tree, create the snapshot ref, re-read master to guard against drift, and
/// fast-forward master to the new commit.
#[tokio::test(flavor = "multi_thread")]
async fn squash_index() {
    let mut github = MockGitHubClient::new();
    expect_get_master(&mut github, ORIGINAL_SHA);
    expect_get_commit(&mut github, ORIGINAL_SHA, TREE_SHA);
    expect_create_commit(&mut github, TREE_SHA, NEW_SHA);
    expect_create_snapshot_ref(&mut github, ORIGINAL_SHA);
    expect_get_master(&mut github, ORIGINAL_SHA); // drift check — still the same
    expect_update_master(&mut github, NEW_SHA);

    let (app, _) = TestApp::init()
        .with_github(github)
        .with_index_location(index_url())
        .with_job_runner()
        .empty()
        .await;

    let conn = app.db_conn().await;
    assert_ok!(jobs::SquashIndex.enqueue(&conn).await);
    app.run_pending_background_jobs().await;
}

/// If `master` has moved between the initial read and the drift check, the
/// job should bail without calling `update_ref`, leaving `master` unchanged
/// on the remote. The snapshot ref created earlier remains as a harmless
/// pointer to the pre-squash HEAD.
#[tokio::test(flavor = "multi_thread")]
async fn squash_index_bails_on_master_drift() {
    const DRIFTED_SHA: &str = "cccccccccccccccccccccccccccccccccccccccc";

    let mut github = MockGitHubClient::new();
    expect_get_master(&mut github, ORIGINAL_SHA);
    expect_get_commit(&mut github, ORIGINAL_SHA, TREE_SHA);
    expect_create_commit(&mut github, TREE_SHA, NEW_SHA);
    expect_create_snapshot_ref(&mut github, ORIGINAL_SHA);
    expect_get_master(&mut github, DRIFTED_SHA); // drift check — master has moved

    // `update_ref` is intentionally not queued; mockall panics on an
    // unexpected call, which is the assertion we want.

    let (app, _) = TestApp::init()
        .with_github(github)
        .with_index_location(index_url())
        .with_job_runner()
        .empty()
        .await;

    let mut conn = app.db_conn().await;
    assert_ok!(jobs::SquashIndex.enqueue(&conn).await);
    let err = app.try_run_pending_background_jobs().await.unwrap_err();
    assert_eq!(err.to_string(), "1 jobs failed");

    // Drain the failed job so the `TestAppInner::drop` empty-queue
    // post-condition is satisfied.
    diesel::delete(background_jobs::table)
        .execute(&mut conn)
        .await
        .unwrap();
}
