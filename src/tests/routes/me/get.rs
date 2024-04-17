use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::views::{EncodablePrivateUser, OwnedCrate};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

impl crate::util::MockCookieUser {
    pub fn show_me(&self) -> UserShowPrivateResponse {
        let url = "/api/v1/me";
        self.get(url).good()
    }
}

#[derive(Deserialize)]
pub struct UserShowPrivateResponse {
    pub user: EncodablePrivateUser,
    pub owned_crates: Vec<OwnedCrate>,
}

#[test]
fn me() {
    let (app, anon, user) = TestApp::init().with_user();

    let response = anon.get::<()>("/api/v1/me");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"this action requires authentication"}]}"###);

    let response = user.get::<()>("/api/v1/me");
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    app.db(|conn| {
        CrateBuilder::new("foo_my_packages", user.as_model().id).expect_build(conn);
    });

    let response = user.get::<()>("/api/v1/me");
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
}

#[test]
fn test_user_owned_crates_doesnt_include_deleted_ownership() {
    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user_model.id).expect_build(conn);
        krate.owner_remove(conn, &user_model.gh_login).unwrap();
    });

    let json = user.show_me();
    assert_eq!(json.owned_crates.len(), 0);
}
