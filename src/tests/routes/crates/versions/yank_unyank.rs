use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, Response, TestApp};
use crate::OkBool;
use http::StatusCode;

pub trait YankRequestHelper {
    /// Yank the specified version of the specified crate and run all pending background jobs
    fn yank(&self, krate_name: &str, version: &str) -> Response<OkBool>;

    /// Unyank the specified version of the specified crate and run all pending background jobs
    fn unyank(&self, krate_name: &str, version: &str) -> Response<OkBool>;
}

impl<T: RequestHelper> YankRequestHelper for T {
    fn yank(&self, krate_name: &str, version: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/yank");
        let response = self.delete(&url);
        self.app().run_pending_background_jobs();
        response
    }

    fn unyank(&self, krate_name: &str, version: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/unyank");
        let response = self.put(&url, &[]);
        self.app().run_pending_background_jobs();
        response
    }
}

#[test]
fn yank_by_a_non_owner_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    let another_user = app.db_new_user("bar");
    let another_user = another_user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_not", another_user.id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let response = token.yank("foo_not", "1.0.0");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "must already be an owner to yank or unyank" }] })
    );
}

#[test]
fn yank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.publish_crate(crate_to_publish).good();

    // Yank it
    token.yank("fyk", "1.0.0").good();

    // Make sure it has one publish and one yank audit action
    let json = anon.show_version("fyk", "1.0.0");
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 2);
    let action = &actions[1];
    assert_eq!(action.action, "yank");
    assert_eq!(action.user.id, token.as_model().user_id);
}

#[test]
fn unyank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk");
    token.publish_crate(crate_to_publish).good();

    // Yank version 1.0.0
    token.yank("fyk", "1.0.0").good();

    // Unyank version 1.0.0
    token.unyank("fyk", "1.0.0").good();

    // Make sure it has one publish, one yank, and one unyank audit action
    let json = anon.show_version("fyk", "1.0.0");
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 3);
    let action = &actions[2];
    assert_eq!(action.action, "unyank");
    assert_eq!(action.user.id, token.as_model().user_id);
}
