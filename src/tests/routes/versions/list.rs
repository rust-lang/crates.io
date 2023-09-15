use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::insta::{self, assert_json_snapshot};
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::versions;
use diesel::{QueryDsl, RunQueryDsl};
use serde_json::Value;

#[test]
fn index() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let url = "/api/v1/versions";

    let json: Value = anon.get(url).good();
    assert_json_snapshot!(json);

    let (v1, v2) = app.db(|conn| {
        CrateBuilder::new("foo_vers_index", user.id)
            .version(VersionBuilder::new("2.0.0").license(Some("MIT")))
            .version(VersionBuilder::new("2.0.1").license(Some("MIT/Apache-2.0")))
            .expect_build(conn);
        let ids: Vec<i32> = versions::table.select(versions::id).load(conn).unwrap();
        (ids[0], ids[1])
    });

    let query = format!("ids[]={v1}&ids[]={v2}");
    let json: Value = anon.get_with_query(url, &query).good();
    assert_json_snapshot!(json, {
        ".versions" => insta::sorted_redaction(),
        ".versions[].id" => insta::any_id_redaction(),
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
        ".versions[].published_by.id" => insta::id_redaction(user.id),
    });
}
