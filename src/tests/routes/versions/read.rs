use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::insta::{self, assert_json_snapshot};
use crate::util::{RequestHelper, TestApp};
use serde_json::Value;

#[test]
fn show_by_id() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show_id", user.id).expect_build(conn);
        VersionBuilder::new("2.0.0")
            .size(1234)
            .expect_build(krate.id, user.id, conn)
    });

    let url = format!("/api/v1/versions/{}", v.id);
    let json: Value = anon.get(&url).good();
    assert_json_snapshot!(json, {
        ".version.id" => insta::id_redaction(v.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => insta::id_redaction(user.id),
    });
}
