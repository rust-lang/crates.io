use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, Response, TestApp};
use crate::OkBool;
use http::StatusCode;

pub trait YankRequestHelper {
    /// Yank the specified version of the specified crate and run all pending background jobs
    async fn yank(&self, krate_name: &str, version: &str) -> Response<OkBool>;

    /// Unyank the specified version of the specified crate and run all pending background jobs
    async fn unyank(&self, krate_name: &str, version: &str) -> Response<OkBool>;
}

impl<T: RequestHelper> YankRequestHelper for T {
    async fn yank(&self, krate_name: &str, version: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/yank");
        let response = self.delete(&url).await;
        self.app().run_pending_background_jobs().await;
        response
    }

    async fn unyank(&self, krate_name: &str, version: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/{version}/unyank");
        let response = self.put(&url, &[] as &[u8]).await;
        self.app().run_pending_background_jobs().await;
        response
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn yank_by_a_non_owner_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    let another_user = app.db_new_user("bar");
    let another_user = another_user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_not", another_user.id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let response = token.yank("foo_not", "1.0.0").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "must already be an owner to yank or unyank" }] })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn yank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    // Yank it
    token.yank("fyk", "1.0.0").await.good();

    // Make sure it has one publish and one yank audit action
    let json = anon.show_version("fyk", "1.0.0").await;
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 2);
    let action = &actions[1];
    assert_eq!(action.action, "yank");
    assert_eq!(action.user.id, token.as_model().user_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn unyank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    // Yank version 1.0.0
    token.yank("fyk", "1.0.0").await.good();

    // Unyank version 1.0.0
    token.unyank("fyk", "1.0.0").await.good();

    // Make sure it has one publish, one yank, and one unyank audit action
    let json = anon.show_version("fyk", "1.0.0").await;
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 3);
    let action = &actions[2];
    assert_eq!(action.action, "unyank");
    assert_eq!(action.user.id, token.as_model().user_id);
}

mod auth {
    use super::*;
    use crate::util::{MockAnonymousUser, MockCookieUser};
    use chrono::{Duration, Utc};
    use crates_io::models::token::{CrateScope, EndpointScope};
    use crates_io::schema::{crates, users, versions};
    use diesel::prelude::*;
    use insta::assert_snapshot;

    const CRATE_NAME: &str = "fyk";
    const CRATE_VERSION: &str = "1.0.0";

    async fn prepare() -> (TestApp, MockAnonymousUser, MockCookieUser) {
        let (app, anon, cookie) = TestApp::full().with_user();

        let pb = PublishBuilder::new(CRATE_NAME, CRATE_VERSION);
        cookie.publish_crate(pb).await.good();

        (app, anon, cookie)
    }

    fn is_yanked(app: &TestApp) -> bool {
        app.db(|conn| {
            versions::table
                .inner_join(crates::table)
                .select(versions::yanked)
                .filter(crates::name.eq(CRATE_NAME))
                .filter(versions::num.eq(CRATE_VERSION))
                .get_result(conn)
                .unwrap()
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unauthenticated() {
        let (app, client, _) = prepare().await;

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);
        assert!(!is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cookie_user() {
        let (app, _, client) = prepare().await;

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user() {
        let (app, _, client) = prepare().await;
        let client = client.db_new_token("test-token");

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_not_expired() {
        let expired_at = Utc::now() + Duration::days(7);

        let (app, _, client) = prepare().await;
        let client =
            client.db_new_scoped_token("test-token", None, None, Some(expired_at.naive_utc()));

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_expired() {
        let expired_at = Utc::now() - Duration::days(7);

        let (app, _, client) = prepare().await;
        let client =
            client.db_new_scoped_token("test-token", None, None, Some(expired_at.naive_utc()));

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"authentication failed"}]}"###);
        assert!(!is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"authentication failed"}]}"###);
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_correct_endpoint_scope() {
        let (app, _, client) = prepare().await;
        let client =
            client.db_new_scoped_token("test-token", None, Some(vec![EndpointScope::Yank]), None);

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_incorrect_endpoint_scope() {
        let (app, _, client) = prepare().await;
        let client = client.db_new_scoped_token(
            "test-token",
            None,
            Some(vec![EndpointScope::PublishUpdate]),
            None,
        );

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_correct_crate_scope() {
        let (app, _, client) = prepare().await;
        let client = client.db_new_scoped_token(
            "test-token",
            Some(vec![CrateScope::try_from(CRATE_NAME).unwrap()]),
            None,
            None,
        );

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_correct_wildcard_crate_scope() {
        let (app, _, client) = prepare().await;
        let wildcard = format!("{}*", CRATE_NAME.chars().next().unwrap());
        let client = client.db_new_scoped_token(
            "test-token",
            Some(vec![CrateScope::try_from(wildcard).unwrap()]),
            None,
            None,
        );

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_incorrect_crate_scope() {
        let (app, _, client) = prepare().await;
        let client = client.db_new_scoped_token(
            "test-token",
            Some(vec![CrateScope::try_from("foo").unwrap()]),
            None,
            None,
        );

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_user_with_incorrect_wildcard_crate_scope() {
        let (app, _, client) = prepare().await;
        let client = client.db_new_scoped_token(
            "test-token",
            Some(vec![CrateScope::try_from("foo*").unwrap()]),
            None,
            None,
        );

        let response = client.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));

        let response = client.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"###);
        assert!(!is_yanked(&app));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn admin() {
        let (app, _, _) = prepare().await;

        let admin = app.db_new_user("admin");

        app.db(|conn| {
            diesel::update(admin.as_model())
                .set(users::is_admin.eq(true))
                .execute(conn)
                .unwrap();
        });

        let response = admin.yank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(is_yanked(&app));

        let response = admin.unyank(CRATE_NAME, CRATE_VERSION).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
        assert!(!is_yanked(&app));
    }
}
