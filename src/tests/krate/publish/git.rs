use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_git_upload_with_conflicts() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.upstream_index().create_empty_commit().unwrap();

    let crate_to_publish = PublishBuilder::new("foo_conflicts", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    assert_snapshot!(app.stored_files().await.join("\n"), @r###"
    crates/foo_conflicts/foo_conflicts-1.0.0.crate
    index/fo/o_/foo_conflicts
    rss/updates.xml
    "###);
}
