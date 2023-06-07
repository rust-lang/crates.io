use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::views::{EncodablePrivateUser, OwnedCrate};

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
    let url = "/api/v1/me";
    let (app, anon) = TestApp::init().empty();
    anon.get::<()>(url).assert_forbidden();

    let user = app.db_new_user("foo");
    let json = user.show_me();

    assert_eq!(json.owned_crates.len(), 0);

    app.db(|conn| {
        CrateBuilder::new("foo_my_packages", user.as_model().id).expect_build(conn);
        assert_eq!(json.user.email, user.as_model().email(conn).unwrap());
    });
    let updated_json = user.show_me();

    assert_eq!(updated_json.owned_crates.len(), 1);
}

#[test]
fn test_user_owned_crates_doesnt_include_deleted_ownership() {
    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user_model.id).expect_build(conn);
        krate
            .owner_remove(app.as_inner(), conn, user_model, &user_model.gh_login)
            .unwrap();
    });

    let json = user.show_me();
    assert_eq!(json.owned_crates.len(), 0);
}
