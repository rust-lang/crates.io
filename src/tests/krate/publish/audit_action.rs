use googletest::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn publish_records_an_audit_action() {
    use crate::models::VersionOwnerAction;
    use crate::tests::builders::PublishBuilder;
    use crate::tests::util::{RequestHelper, TestApp};

    let (app, anon, _, token) = TestApp::full().with_token();

    app.db(|conn| assert!(VersionOwnerAction::all(conn).unwrap().is_empty()));

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    // Make sure it has one publish audit action
    let json = anon.show_version("fyk", "1.0.0").await;
    let actions = json.version.audit_actions;

    assert_that!(actions, len(eq(1)));
    let action = &actions[0];
    assert_eq!(action.action, "publish");
    assert_eq!(action.user.id, token.as_model().user_id);
}
