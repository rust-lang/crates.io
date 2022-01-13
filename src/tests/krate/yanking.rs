use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crate::OkBool;
use http::StatusCode;

impl crate::util::MockTokenUser {
    /// Yank the specified version of the specified crate and run all pending background jobs
    fn yank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/yank");
        let response = self.delete(&url);
        self.app().run_pending_background_jobs();
        response
    }

    /// Unyank the specified version of the specified crate and run all pending background jobs
    fn unyank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/unyank");
        let response = self.put(&url, &[]);
        self.app().run_pending_background_jobs();
        response
    }
}

impl crate::util::MockCookieUser {
    /// Yank the specified version of the specified crate and run all pending background jobs
    fn yank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/yank");
        let response = self.delete(&url);
        self.app().run_pending_background_jobs();
        response
    }

    /// Unyank the specified version of the specified crate and run all pending background jobs
    fn unyank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/unyank");
        let response = self.put(&url, &[]);
        self.app().run_pending_background_jobs();
        response
    }
}

#[test]
#[allow(unknown_lints, clippy::bool_assert_comparison)] // for claim::assert_some_eq! with bool
fn yank_works_as_intended() {
    let (app, anon, cookie, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    // make sure it's not yanked
    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    token.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    token.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    cookie.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    cookie.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);
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
fn yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.enqueue_publish(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    token.yank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    token.unyank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.enqueue_publish(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn yank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();

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
    token.enqueue_publish(crate_to_publish).good();

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
