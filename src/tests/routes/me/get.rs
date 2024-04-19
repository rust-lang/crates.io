use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::views::{EncodablePrivateUser, OwnedCrate};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

impl crate::util::MockCookieUser {
    pub async fn async_show_me(&self) -> UserShowPrivateResponse {
        let url = "/api/v1/me";
        self.async_get(url).await.good()
    }
}

#[derive(Deserialize)]
pub struct UserShowPrivateResponse {
    pub user: EncodablePrivateUser,
    pub owned_crates: Vec<OwnedCrate>,
}

#[tokio::test(flavor = "multi_thread")]
async fn me() {
    let (app, anon, user) = TestApp::init().with_user();

    let response = anon.async_get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);

    let response = user.async_get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    app.db(|conn| {
        CrateBuilder::new("foo_my_packages", user.as_model().id).expect_build(conn);
    });

    let response = user.async_get::<()>("/api/v1/me").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_owned_crates_doesnt_include_deleted_ownership() {
    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user_model.id).expect_build(conn);
        krate.owner_remove(conn, &user_model.gh_login).unwrap();
    });

    let json = user.async_show_me().await;
    assert_eq!(json.owned_crates.len(), 0);
}
