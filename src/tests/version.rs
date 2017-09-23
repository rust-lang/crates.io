extern crate diesel;
extern crate serde_json;

use serde_json::Value;

use conduit::{Handler, Method};
use self::diesel::prelude::*;

use cargo_registry::version::EncodableVersion;
use cargo_registry::schema::versions;

#[derive(Deserialize)]
struct VersionList {
    versions: Vec<EncodableVersion>,
}
#[derive(Deserialize)]
struct VersionResponse {
    version: EncodableVersion,
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/versions");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 0);

    let (v1, v2) = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_vers_index", u.id)
            .version(::VersionBuilder::new("2.0.0").license(Some("MIT")))
            .version(::VersionBuilder::new("2.0.1").license(Some("MIT/Apache-2.0")))
            .expect_build(&conn);
        let ids = versions::table
            .select(versions::id)
            .load::<i32>(&*conn)
            .unwrap();
        (ids[0], ids[1])
    };
    req.with_query(&format!("ids[]={}&ids[]={}", v1, v2));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 2);

    for v in &json.versions {
        match v.num.as_ref() {
            "2.0.0" => assert_eq!(v.license, Some(String::from("MIT"))),
            "2.0.1" => assert_eq!(v.license, Some(String::from("MIT/Apache-2.0"))),
            _ => panic!("unexpected version"),
        }
    }
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/versions");
    let v = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("foo_vers_show", user.id).expect_build(&conn);
        ::new_version(krate.id, "2.0.0").save(&conn, &[]).unwrap()
    };
    req.with_path(&format!("/api/v1/versions/{}", v.id));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionResponse = ::json(&mut response);
    assert_eq!(json.version.id, v.id);
}

#[test]
fn authors() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_authors/1.0.0/authors",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c = ::CrateBuilder::new("foo_authors", u.id).expect_build(&conn);
        ::new_version(c.id, "1.0.0").save(&conn, &[]).unwrap();
    }
    let mut response = ok_resp!(middle.call(&mut req));
    let mut data = Vec::new();
    response.body.write_body(&mut data).unwrap();
    let s = ::std::str::from_utf8(&data).unwrap();
    let json: Value = serde_json::from_str(&s).unwrap();
    let json = json.as_object().unwrap();
    assert!(json.contains_key(&"users".to_string()));
}
