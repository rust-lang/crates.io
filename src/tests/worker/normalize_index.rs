use crate::util::TestApp;
use claims::assert_ok;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;
use insta::assert_json_snapshot;

/// An unnormalized index entry: deps out of alphabetical order, one dep
/// with a `null` kind, one dep with an empty string inside `features`.
const UNNORMALIZED_ENTRY: &str = concat!(
    r#"{"name":"foo","vers":"1.0.0","deps":[{"name":"zeta","req":"^1","features":["x",""],"optional":false,"default_features":true,"target":null,"kind":null},{"name":"alpha","req":"^1","features":[],"optional":false,"default_features":true,"target":null,"kind":null}],"cksum":"0","features":{},"yanked":false}"#,
    "\n",
);

/// `NormalizeIndex` should rewrite unnormalized entries (sort deps, set null
/// `kind` to `normal`, drop empty feature strings) and push the rewrite as a
/// single commit on `master`.
#[tokio::test(flavor = "multi_thread")]
async fn normalize_index() {
    let (app, _, _, _) = TestApp::full().with_git_index().with_token().await;
    let conn = app.db_conn().await;
    let upstream = app.upstream_index();

    upstream.write_file("3/f/foo", UNNORMALIZED_ENTRY).unwrap();
    let before = upstream.list_commits().unwrap().len();

    assert_ok!(jobs::NormalizeIndex::new(false).enqueue(&conn).await);
    app.run_pending_background_jobs().await;

    let commits = upstream.list_commits().unwrap();
    assert_eq!(commits.len(), before + 1);
    assert!(
        commits
            .last()
            .unwrap()
            .starts_with("Normalize index format")
    );

    // The normalized entry should:
    // - have `deps` sorted alphabetically (alpha before zeta)
    // - have null `kind` values rewritten to `"normal"`
    // - have the empty string stripped from `zeta`'s `features`
    let normalized = upstream.read_file("3/f/foo").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&normalized).unwrap();
    assert_json_snapshot!(parsed, @r#"
    {
      "cksum": "0",
      "deps": [
        {
          "default_features": true,
          "features": [],
          "kind": "normal",
          "name": "alpha",
          "optional": false,
          "req": "^1",
          "target": null
        },
        {
          "default_features": true,
          "features": [
            "x"
          ],
          "kind": "normal",
          "name": "zeta",
          "optional": false,
          "req": "^1",
          "target": null
        }
      ],
      "features": {},
      "name": "foo",
      "vers": "1.0.0",
      "yanked": false
    }
    "#);
}

/// Dry-run mode should push the normalization commit to the
/// `normalization-dry-run` branch, leaving `master` untouched.
#[tokio::test(flavor = "multi_thread")]
async fn normalize_index_dry_run() {
    let (app, _, _, _) = TestApp::full().with_git_index().with_token().await;
    let conn = app.db_conn().await;
    let upstream = app.upstream_index();

    upstream.write_file("3/f/foo", UNNORMALIZED_ENTRY).unwrap();
    let master_before = upstream.list_commits().unwrap();

    assert_ok!(jobs::NormalizeIndex::new(true).enqueue(&conn).await);
    app.run_pending_background_jobs().await;

    // master is untouched
    assert_eq!(upstream.list_commits().unwrap(), master_before);

    // the `normalization-dry-run` branch now exists and has the normalize commit
    let bare = upstream.repository.lock().unwrap();
    let dry_run_ref = bare
        .find_reference("refs/heads/normalization-dry-run")
        .unwrap();
    let commit = bare.find_commit(dry_run_ref.target().unwrap()).unwrap();
    assert!(
        commit
            .message()
            .unwrap()
            .starts_with("Normalize index format")
    );
}
