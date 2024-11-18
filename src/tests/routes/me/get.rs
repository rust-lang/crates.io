use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::{EncodablePrivateUser, OwnedCrate};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

impl crate::tests::util::MockCookieUser {
    pub async fn show_me(&self) -> UserShowPrivateResponse {
        let url = "/api/v1/me";
        self.get(url).await.good()
    }
}

#[derive(Deserialize)]
pub struct UserShowPrivateResponse {
    pub user: EncodablePrivateUser,
    pub owned_crates: Vec<OwnedCrate>,
}

#[tokio::test(flavor = "multi_thread")]
async fn me() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();

    let response = anon.get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    let response = user.get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    CrateBuilder::new("foo_my_packages", user.as_model().id).expect_build(&mut conn);

    let response = user.get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_owned_crates_doesnt_include_deleted_ownership() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();
    let user_model = user.as_model();

    let krate = CrateBuilder::new("foo_my_packages", user_model.id).expect_build(&mut conn);
    krate.owner_remove(&mut conn, &user_model.gh_login).unwrap();

    let json = user.show_me().await;
    assert_eq!(json.owned_crates.len(), 0);
}
