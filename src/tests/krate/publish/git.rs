use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_git_upload_with_conflicts() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.upstream_index().create_empty_commit().unwrap();

    let crate_to_publish = PublishBuilder::new("foo_conflicts", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    let expected_files = vec![
        "crates/foo_conflicts/foo_conflicts-1.0.0.crate",
        "index/fo/o_/foo_conflicts",
    ];
    assert_eq!(app.stored_files().await, expected_files);
}
