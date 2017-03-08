use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::{self, File};

use conduit::{Handler, Request, Method};

use git2;
use rustc_serialize::json;
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::dependency::EncodableDependency;
use cargo_registry::download::EncodableVersionDownload;
use cargo_registry::keyword::{Keyword, EncodableKeyword};
use cargo_registry::krate::{Crate, EncodableCrate};
use cargo_registry::upload as u;
use cargo_registry::user::EncodableUser;
use cargo_registry::version::EncodableVersion;
use cargo_registry::category::Category;

#[derive(RustcDecodable)]
struct CrateList { crates: Vec<EncodableCrate>, meta: CrateMeta }
#[derive(RustcDecodable)]
struct VersionsList { versions: Vec<EncodableVersion> }
#[derive(RustcDecodable)]
struct CrateMeta { total: i32 }
#[derive(RustcDecodable)]
struct GitCrate { name: String, vers: String, deps: Vec<String>, cksum: String }
#[derive(RustcDecodable)]
struct Warnings { invalid_categories: Vec<String>, invalid_badges: Vec<String> }
#[derive(RustcDecodable)]
struct GoodCrate { krate: EncodableCrate, warnings: Warnings }
#[derive(RustcDecodable)]
struct CrateResponse { krate: EncodableCrate, versions: Vec<EncodableVersion>, keywords: Vec<EncodableKeyword> }
#[derive(RustcDecodable)]
struct Deps { dependencies: Vec<EncodableDependency> }
#[derive(RustcDecodable)]
struct RevDeps { dependencies: Vec<EncodableDependency>, meta: CrateMeta }
#[derive(RustcDecodable)]
struct Downloads { version_downloads: Vec<EncodableVersionDownload> }

fn new_crate(name: &str) -> u::NewCrate {
    u::NewCrate {
        name: u::CrateName(name.to_string()),
        vers: u::CrateVersion(semver::Version::parse("1.1.0").unwrap()),
        features: HashMap::new(),
        deps: Vec::new(),
        authors: vec!["foo".to_string()],
        description: Some("desc".to_string()),
        homepage: None,
        documentation: None,
        readme: None,
        keywords: None,
        categories: None,
        license: Some("MIT".to_string()),
        license_file: None,
        repository: None,
        badges: None,
    }
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);

    let krate = ::krate("fooindex");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, krate.clone());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.crates[0].name, krate.name);
    assert_eq!(json.crates[0].id, krate.name);
}

#[test]
fn index_queries() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let u = ::mock_user(&mut req, ::user("foo"));
    let mut krate = ::krate("foo_index_queries");
    krate.readme = Some("readme".to_string());
    krate.description = Some("description".to_string());
    let (krate, _) = ::mock_crate(&mut req, krate.clone());
    let krate2 = ::krate("BAR_INDEX_QUERIES");
    let (krate2, _) = ::mock_crate(&mut req, krate2.clone());
    Keyword::update_crate(req.tx().unwrap(), &krate, &["kw1".into()]).unwrap();
    Keyword::update_crate(req.tx().unwrap(), &krate2, &["KW1".into()]).unwrap();

    let mut response = ok_resp!(middle.call(req.with_query("q=baz")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 0);

    // All of these fields should be indexed/searched by the queries
    let mut response = ok_resp!(middle.call(req.with_query("q=foo")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("q=kw1")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 2);
    let mut response = ok_resp!(middle.call(req.with_query("q=readme")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("q=description")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 1);

    let query = format!("user_id={}", u.id);
    let mut response = ok_resp!(middle.call(req.with_query(&query)));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 2);
    let mut response = ok_resp!(middle.call(req.with_query("user_id=0")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);

    let mut response = ok_resp!(middle.call(req.with_query("letter=F")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);
    let mut response = ok_resp!(middle.call(req.with_query("letter=B")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);
    let mut response = ok_resp!(middle.call(req.with_query("letter=b")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);
    let mut response = ok_resp!(middle.call(req.with_query("letter=c")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);

    let mut response = ok_resp!(middle.call(req.with_query("keyword=kw1")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 2);
    let mut response = ok_resp!(middle.call(req.with_query("keyword=KW1")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 2);
    let mut response = ok_resp!(middle.call(req.with_query("keyword=kw2")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);

    ::mock_category(&mut req, "cat1", "cat1");
    ::mock_category(&mut req, "cat1::bar", "cat1::bar");
    Category::update_crate(req.tx().unwrap(), &krate, &["cat1".to_string(),
                            "cat1::bar".to_string()]).unwrap();
    let mut response = ok_resp!(middle.call(req.with_query("category=cat1")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("category=cat1::bar")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("keyword=cat2")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 0);
    assert_eq!(cl.meta.total, 0);
}

#[test]
fn exact_match_first_on_queries() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let _ = ::mock_user(&mut req, ::user("foo"));
    let mut krate = ::krate("foo_exact");
    krate.description = Some("bar_exact baz_exact".to_string());
    let (_, _) = ::mock_crate(&mut req, krate.clone());
    let mut krate2 = ::krate("bar_exact");
    krate2.description = Some("foo_exact baz_exact foo_exact baz_exact".to_string());
    let (_, _) = ::mock_crate(&mut req, krate2.clone());
    let mut krate3 = ::krate("baz_exact");
    krate3.description = Some("foo_exact bar_exact foo_exact bar_exact foo_exact bar_exact".to_string());
    let (_, _) = ::mock_crate(&mut req, krate3.clone());
    let mut krate4 = ::krate("other_exact");
    krate4.description = Some("other_exact".to_string());
    let (_, _) = ::mock_crate(&mut req, krate4.clone());

    let mut response = ok_resp!(middle.call(req.with_query("q=foo_exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "foo_exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "bar_exact");

    let mut response = ok_resp!(middle.call(req.with_query("q=bar_exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "foo_exact");

    let mut response = ok_resp!(middle.call(req.with_query("q=baz_exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_exact");
    assert_eq!(json.crates[1].name, "bar_exact");
    assert_eq!(json.crates[2].name, "foo_exact");
}

#[test]
fn exact_match_on_queries_with_sort() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let _ = ::mock_user(&mut req, ::user("foo"));
    let mut krate = ::krate("foo_sort");
    krate.description = Some("bar_sort baz_sort const".to_string());
    krate.downloads = 50;
    let (k, _) = ::mock_crate(&mut req, krate.clone());
    let mut krate2 = ::krate("bar_sort");
    krate2.description = Some("foo_sort baz_sort foo_sort baz_sort const".to_string());
    krate2.downloads = 3333;
    let (k2, _) = ::mock_crate(&mut req, krate2.clone());
    let mut krate3 = ::krate("baz_sort");
    krate3.description = Some("foo_sort bar_sort foo_sort bar_sort foo_sort bar_sort const".to_string());
    krate3.downloads = 100000;
    let (k3, _) = ::mock_crate(&mut req, krate3.clone());
    let mut krate4 = ::krate("other_sort");
    krate4.description = Some("other_sort const".to_string());
    krate4.downloads = 999999;
    let (k4, _) = ::mock_crate(&mut req, krate4.clone());

    {
        let tx = req.tx().unwrap();
        tx.execute("UPDATE crates set downloads = $1
                    WHERE id = $2", &[&krate.downloads, &k.id]).unwrap();
        tx.execute("UPDATE crates set downloads = $1
                    WHERE id = $2", &[&krate2.downloads, &k2.id]).unwrap();
        tx.execute("UPDATE crates set downloads = $1
                    WHERE id = $2", &[&krate3.downloads, &k3.id]).unwrap();
        tx.execute("UPDATE crates set downloads = $1
                    WHERE id = $2", &[&krate4.downloads, &k4.id]).unwrap();
    }

    let mut response = ok_resp!(middle.call(req.with_query("q=foo_sort&sort=downloads")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");

    let mut response = ok_resp!(middle.call(req.with_query("q=bar_sort&sort=downloads")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "foo_sort");

    let mut response = ok_resp!(middle.call(req.with_query("q=baz_sort&sort=downloads")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_sort");
    assert_eq!(json.crates[1].name, "bar_sort");
    assert_eq!(json.crates[2].name, "foo_sort");

    let mut response = ok_resp!(middle.call(req.with_query("q=const&sort=downloads")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "other_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "foo_sort");
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_show");
    ::mock_user(&mut req, ::user("foo"));
    let mut krate = ::krate("foo_show");
    krate.description = Some(format!("description"));
    krate.documentation = Some(format!("https://example.com"));
    krate.homepage = Some(format!("http://example.com"));
    let (krate, _) = ::mock_crate(&mut req, krate.clone());
    Keyword::update_crate(req.tx().unwrap(), &krate, &["kw1".into()]).unwrap();

    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.id, krate.name);
    assert_eq!(json.krate.description, krate.description);
    assert_eq!(json.krate.homepage, krate.homepage);
    assert_eq!(json.krate.documentation, krate.documentation);
    assert_eq!(json.krate.keywords, Some(vec!["kw1".into()]));
    let versions = json.krate.versions.as_ref().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(json.versions.len(), 1);
    assert_eq!(json.versions[0].id, versions[0]);
    assert_eq!(json.versions[0].krate, json.krate.id);
    assert_eq!(json.versions[0].num, "1.0.0".to_string());
    let suffix = "/api/v1/crates/foo_show/1.0.0/download";
    assert!(json.versions[0].dl_path.ends_with(suffix),
            "bad suffix {}", json.versions[0].dl_path);
    assert_eq!(1, json.keywords.len());
    assert_eq!("kw1".to_string(), json.keywords[0].id);
}

#[test]
fn versions() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_versions/versions");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_versions"));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionsList = ::json(&mut response);
    assert_eq!(json.versions.len(), 1);
}

#[test]
fn new_wrong_token() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    bad_resp!(middle.call(&mut req));
    drop(req);

    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    req.header("Authorization", "bad");
    bad_resp!(middle.call(&mut req));
    drop(req);

    let mut req = ::new_req(app, "foo", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    ::logout(&mut req);
    req.header("Authorization", "bad");
    bad_resp!(middle.call(&mut req));
}

#[test]
fn new_bad_names() {
    fn bad_name(name: &str) {
        println!("testing: `{}`", name);
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app, name, "1.0.0");
        ::mock_user(&mut req, ::user("foo"));
        ::logout(&mut req);
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("invalid crate name"),
                "{:?}", json.errors);
    }

    bad_name("");
    bad_name("foo bar");
}

#[test]
fn new_krate() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_new", "1.0.0");
    let user = ::mock_user(&mut req, ::user("foo"));
    ::logout(&mut req);
    req.header("Authorization", &user.api_token);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_with_reserved_name() {
    fn test_bad_name(name: &str) {
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app, name, "1.0.0");
        ::mock_user(&mut req, ::user("foo"));
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("cannot upload a crate with a reserved name"));
    }

    test_bad_name("std");
    test_bad_name("STD");
    test_bad_name("compiler-rt");
    test_bad_name("compiler_rt");
    test_bad_name("coMpiLer_Rt");
}

#[test]
fn new_krate_weird_version() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_weird", "0.0.0-pre");
    let user = ::mock_user(&mut req, ::user("foo"));
    ::logout(&mut req);
    req.header("Authorization", &user.api_token);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_weird");
    assert_eq!(json.krate.max_version, "0.0.0-pre");
}

#[test]
fn new_krate_with_dependency() {
    let (_b, app, middle) = ::app();
    let dep = u::CrateDependency {
        name: u::CrateName("foo_dep".to_string()),
        optional: false,
        default_features: true,
        features: Vec::new(),
        version_req: u::CrateVersionReq(semver::VersionReq::parse(">= 0").unwrap()),
        target: None,
        kind: None,
    };
    let mut req = ::new_req_full(app, ::krate("new_dep"), "1.0.0", vec![dep]);
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_dep"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn new_krate_with_wildcard_dependency() {
    let (_b, app, middle) = ::app();
    let dep = u::CrateDependency {
        name: u::CrateName("foo_wild".to_string()),
        optional: false,
        default_features: true,
        features: Vec::new(),
        version_req: u::CrateVersionReq(semver::VersionReq::parse("*").unwrap()),
        target: None,
        kind: None,
    };
    let mut req = ::new_req_full(app, ::krate("new_wild"), "1.0.0", vec![dep]);
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_wild"));
    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("dependency constraints"), "{:?}", json.errors);
}

#[test]
fn new_krate_twice() {
    let (_b, app, middle) = ::app();
    let mut krate = ::krate("foo_twice");
    krate.description = Some("description".to_string());
    let mut req = ::new_req_full(app, krate.clone(), "2.0.0", Vec::new());
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_twice"));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.description, krate.description);
}

#[test]
fn new_krate_wrong_user() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app, "foo_wrong", "2.0.0");

    // Create the 'foo' crate with one user
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_wrong"));

    // But log in another
    ::mock_user(&mut req, ::user("bar"));

    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("another user"),
            "{:?}", json.errors);
}

#[test]
fn new_krate_bad_name() {
    let (_b, app, middle) = ::app();

    {
        let mut req = ::new_req(app.clone(), "snow☃", "2.0.0");
        ::mock_user(&mut req, ::user("foo"));
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("invalid crate name"),
                "{:?}", json.errors);
    }
    {
        let mut req = ::new_req(app, "áccênts", "2.0.0");
        ::mock_user(&mut req, ::user("foo"));
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("invalid crate name"),
                "{:?}", json.errors);
    }
}

#[test]
fn new_crate_owner() {
    #[derive(RustcDecodable)] struct O { ok: bool }

    let (_b, app, middle) = ::app();

    // Create a crate under one user
    let mut req = ::new_req(app.clone(), "foo_owner", "1.0.0");
    let u2 = ::mock_user(&mut req, ::user("bar"));
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    // Flag the second user as an owner
    let body = r#"{"users":["bar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_owner/owners")
                                               .with_method(Method::Put)
                                               .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);
    bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_owner/owners")
                             .with_method(Method::Put)
                             .with_body(body.as_bytes())));

    // Make sure this shows up as one of their crates.
    let query = format!("user_id={}", u2.id);
    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/crates")
                                               .with_method(Method::Get)
                                               .with_query(&query)));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);

    // And upload a new crate as the first user
    let body = ::new_req_body_version_2(::krate("foo_owner"));
    req.mut_extensions().insert(u2);
    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
                                               .with_method(Method::Put)
                                               .with_body(&body)));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn valid_feature_names() {
    assert!(Crate::valid_feature_name("foo"));
    assert!(!Crate::valid_feature_name(""));
    assert!(!Crate::valid_feature_name("/"));
    assert!(!Crate::valid_feature_name("%/%"));
    assert!(Crate::valid_feature_name("a/a"));
}

#[test]
fn new_krate_too_big() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_big", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let body = ::new_crate_to_body(&new_crate("foo_big"), &[b'a'; 2000]);
    bad_resp!(middle.call(req.with_body(&body)));
}

#[test]
fn new_krate_too_big_but_whitelisted() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_whitelist", "1.1.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut krate = ::krate("foo_whitelist");
    krate.max_upload_size = Some(2 * 1000 * 1000);
    ::mock_crate(&mut req, krate);
    let body = ::new_crate_to_body(&new_crate("foo_whitelist"), &[b'a'; 2000]);
    let mut response = ok_resp!(middle.call(req.with_body(&body)));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn new_krate_duplicate_version() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_dupe", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_dupe"));
    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("already uploaded"),
            "{:?}", json.errors);
}

#[test]
fn new_crate_similar_name() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_similar", "1.1.0");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("Foo_similar"));
    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("previously named"),
            "{:?}", json.errors);
}

#[test]
fn new_crate_similar_name_hyphen() {
    {
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app, "foo-bar-hyphen", "1.1.0");
        ::mock_user(&mut req, ::user("foo"));
        ::mock_crate(&mut req, ::krate("foo_bar_hyphen"));
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("previously named"),
                "{:?}", json.errors);
    }
    {
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app, "foo_bar_underscore", "1.1.0");
        ::mock_user(&mut req, ::user("foo"));
        ::mock_crate(&mut req, ::krate("foo-bar-underscore"));
        let json = bad_resp!(middle.call(&mut req));
        assert!(json.errors[0].detail.contains("previously named"),
                "{:?}", json.errors);
    }
}

#[test]
fn new_krate_git_upload() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "fgt", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let path = ::git::checkout().join("3/f/fgt");
    assert!(path.exists());
    let mut contents = String::new();
    File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
    let p: GitCrate = json::decode(&contents).unwrap();
    assert_eq!(p.name, "fgt");
    assert_eq!(p.vers, "1.0.0");
    assert!(p.deps.is_empty());
    assert_eq!(p.cksum,
               "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn new_krate_git_upload_appends() {
    let (_b, app, middle) = ::app();
    let path = ::git::checkout().join("3/f/fpp");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    File::create(&path).unwrap().write_all(
        br#"{"name":"FPP","vers":"0.0.1","deps":[],"cksum":"3j3"}
"#).unwrap();

    let mut req = ::new_req(app, "FPP", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let mut contents = String::new();
    File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
    let mut lines = contents.lines();
    let p1: GitCrate = json::decode(lines.next().unwrap().trim()).unwrap();
    let p2: GitCrate = json::decode(lines.next().unwrap().trim()).unwrap();
    assert!(lines.next().is_none());
    assert_eq!(p1.name, "FPP");
    assert_eq!(p1.vers, "0.0.1");
    assert!(p1.deps.is_empty());
    assert_eq!(p2.name, "FPP");
    assert_eq!(p2.vers, "1.0.0");
    assert!(p2.deps.is_empty());
}

#[test]
fn new_krate_git_upload_with_conflicts() {
    let (_b, app, middle) = ::app();

    {
        let repo = git2::Repository::open(&::git::bare()).unwrap();
        let target = repo.head().unwrap().target().unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.find_commit(target).unwrap();
        let tree = repo.find_tree(parent.tree_id()).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "empty commit", &tree,
                    &[&parent]).unwrap();
    }

    let mut req = ::new_req(app, "foo_conflicts", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn new_krate_dependency_missing() {
    let (_b, app, middle) = ::app();
    let dep = u::CrateDependency {
        optional: false,
        default_features: true,
        name: u::CrateName("bar_missing".to_string()),
        features: Vec::new(),
        version_req: u::CrateVersionReq(semver::VersionReq::parse(">= 0.0.0").unwrap()),
        target: None,
        kind: None,
    };
    let mut req = ::new_req_full(app, ::krate("foo_missing"), "1.0.0", vec![dep]);
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    let json = ::json::<::Bad>(&mut response);
    assert!(json.errors[0].detail
                .contains("no known crate named `bar_missing`"));
}

#[test]
fn summary_doesnt_die() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/summary");
    ok_resp!(middle.call(&mut req));
}

#[test]
fn download() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_download/1.0.0/download");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_download"));
    let resp = t_resp!(middle.call(&mut req));
    assert_eq!(resp.status.0, 302);

    req.with_path("/api/v1/crates/foo_download/1.0.0/downloads");
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);

    req.with_path("/api/v1/crates/FOO_DOWNLOAD/1.0.0/download");
    let resp = t_resp!(middle.call(&mut req));
    assert_eq!(resp.status.0, 302);

    req.with_path("/api/v1/crates/FOO_DOWNLOAD/1.0.0/downloads");
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);
}

#[test]
fn download_bad() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_bad/0.1.0/download");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_bad"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<::Bad>(&mut response);
}

#[test]
fn dependencies() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_deps/1.0.0/dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (_, v) = ::mock_crate(&mut req, ::krate("foo_deps"));
    let (c, _) = ::mock_crate(&mut req, ::krate("bar_deps"));
    ::mock_dep(&mut req, &v, &c, None);

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<Deps>(&mut response);
    assert_eq!(deps.dependencies[0].crate_id, "bar_deps");

    req.with_path("/api/v1/crates/foo_deps/1.0.2/dependencies");
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<::Bad>(&mut response);
}

#[test]
fn following() {
    #[derive(RustcDecodable)] struct F { following: bool }
    #[derive(RustcDecodable)] struct O { ok: bool }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_following/following");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_following"));

    let mut response = ok_resp!(middle.call(&mut req));
    assert!(!::json::<F>(&mut response).following);

    req.with_path("/api/v1/crates/foo_following/follow")
       .with_method(Method::Put);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(::json::<O>(&mut response).ok);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(::json::<O>(&mut response).ok);

    req.with_path("/api/v1/crates/foo_following/following")
       .with_method(Method::Get);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(::json::<F>(&mut response).following);

    req.with_path("/api/v1/crates")
       .with_query("following=1");
    let mut response = ok_resp!(middle.call(&mut req));
    let l = ::json::<CrateList>(&mut response);
    assert_eq!(l.crates.len(), 1);

    req.with_path("/api/v1/crates/foo_following/follow")
       .with_method(Method::Delete);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(::json::<O>(&mut response).ok);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(::json::<O>(&mut response).ok);

    req.with_path("/api/v1/crates/foo_following/following")
       .with_method(Method::Get);
    let mut response = ok_resp!(middle.call(&mut req));
    assert!(!::json::<F>(&mut response).following);

    req.with_path("/api/v1/crates")
       .with_query("following=1")
       .with_method(Method::Get);
    let mut response = ok_resp!(middle.call(&mut req));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);
}

#[test]
fn owners() {
    #[derive(RustcDecodable)] struct R { users: Vec<EncodableUser> }
    #[derive(RustcDecodable)] struct O { ok: bool }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_owners/owners");
    let other = ::user("foobar");
    ::mock_user(&mut req, other);
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_owners"));

    let mut response = ok_resp!(middle.call(&mut req));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Put)
                                               .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 2);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete)
                                               .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)));
    let r: R = ::json(&mut response);
    assert_eq!(r.users.len(), 1);

    let body = r#"{"users":["foo"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Delete)
                                               .with_body(body.as_bytes())));
    ::json::<::Bad>(&mut response);

    let body = r#"{"users":["foobar"]}"#;
    let mut response = ok_resp!(middle.call(req.with_method(Method::Put)
                                               .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);
}

#[test]
fn yank() {
    #[derive(RustcDecodable)] struct O { ok: bool }
    #[derive(RustcDecodable)] struct V { version: EncodableVersion }
    let (_b, app, middle) = ::app();
    let path = ::git::checkout().join("3/f/fyk");

    // Upload a new crate, putting it in the git index
    let mut req = ::new_req(app, "fyk", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);
    let mut contents = String::new();
    File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
    assert!(contents.contains("\"yanked\":false"));

    // make sure it's not yanked
    let mut r = ok_resp!(middle.call(req.with_method(Method::Get)
                                        .with_path("/api/v1/crates/fyk/1.0.0")));
    assert!(!::json::<V>(&mut r).version.yanked);

    // yank it
    let mut r = ok_resp!(middle.call(req.with_method(Method::Delete)
                                        .with_path("/api/v1/crates/fyk/1.0.0/yank")));
    assert!(::json::<O>(&mut r).ok);
    let mut contents = String::new();
    File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
    assert!(contents.contains("\"yanked\":true"));
    let mut r = ok_resp!(middle.call(req.with_method(Method::Get)
                                        .with_path("/api/v1/crates/fyk/1.0.0")));
    assert!(::json::<V>(&mut r).version.yanked);

    // un-yank it
    let mut r = ok_resp!(middle.call(req.with_method(Method::Put)
                                        .with_path("/api/v1/crates/fyk/1.0.0/unyank")));
    assert!(::json::<O>(&mut r).ok);
    let mut contents = String::new();
    File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
    assert!(contents.contains("\"yanked\":false"));
    let mut r = ok_resp!(middle.call(req.with_method(Method::Get)
                                        .with_path("/api/v1/crates/fyk/1.0.0")));
    assert!(!::json::<V>(&mut r).version.yanked);
}

#[test]
fn yank_not_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Delete, "/api/v1/crates/foo_not/1.0.0/yank");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_not"));
    ::mock_user(&mut req, ::user("bar"));
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<::Bad>(&mut response);
}

#[test]
fn yank_max_version() {
    #[derive(RustcDecodable)]
    struct O {
        ok: bool,
    }
    let (_b, app, middle) = ::app();

    // Upload a new crate
    let mut req = ::new_req(app, "fyk_max", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));

    // double check the max version
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let body = ::new_req_body_version_2(::krate("fyk_max"));
    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
        .with_method(Method::Put)
        .with_body(&body)));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Delete)
        .with_path("/api/v1/crates/fyk_max/1.0.0/yank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Put)
        .with_path("/api/v1/crates/fyk_max/1.0.0/unyank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Delete)
        .with_path("/api/v1/crates/fyk_max/2.0.0/yank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Delete)
        .with_path("/api/v1/crates/fyk_max/1.0.0/yank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Put)
        .with_path("/api/v1/crates/fyk_max/2.0.0/unyank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Put)
        .with_path("/api/v1/crates/fyk_max/1.0.0/unyank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_yank_max_version() {
    #[derive(RustcDecodable)]
    struct O {
        ok: bool,
    }
    let (_b, app, middle) = ::app();

    // Upload a new crate
    let mut req = ::new_req(app, "fyk_max", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));

    // double check the max version
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Delete)
        .with_path("/api/v1/crates/fyk_max/1.0.0/yank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let body = ::new_req_body_version_2(::krate("fyk_max"));
    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
        .with_method(Method::Put)
        .with_body(&body)));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(middle.call(req.with_method(Method::Put)
        .with_path("/api/v1/crates/fyk_max/1.0.0/unyank")));
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(middle.call(req.with_method(Method::Get)
        .with_path("/api/v1/crates/fyk_max")));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn bad_keywords() {
    let (_b, app, middle) = ::app();
    {
        let krate = ::krate("foo_bad_key");
        let kws = vec!["super-long-keyword-name-oh-no".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::mock_user(&mut req, ::user("foo"));
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key2");
        let kws = vec!["?@?%".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::mock_user(&mut req, ::user("foo"));
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key_3");
        let kws = vec!["?@?%".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::mock_user(&mut req, ::user("foo"));
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key4");
        let kws = vec!["áccênts".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::mock_user(&mut req, ::user("foo"));
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
}

#[test]
fn good_categories() {
    let (_b, app, middle) = ::app();
    let krate = ::krate("foo_good_cat");
    let cats = vec!["cat1".into()];
    let mut req = ::new_req_with_categories(app, krate, "1.0.0", cats);
    ::mock_category(&mut req, "cat1", "cat1");
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_good_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories.len(), 0);
}

#[test]
fn ignored_categories() {
    let (_b, app, middle) = ::app();
    let krate = ::krate("foo_ignored_cat");
    let cats = vec!["bar".into()];
    let mut req = ::new_req_with_categories(app, krate, "1.0.0", cats);
    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_ignored_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories, vec!["bar".to_string()]);
}

#[test]
fn good_badges() {
    let krate = ::krate("foobadger");
    let mut badges = HashMap::new();
    let mut badge_attributes = HashMap::new();
    badge_attributes.insert(
        String::from("repository"),
        String::from("rust-lang/crates.io")
    );
    badges.insert(String::from("travis-ci"), badge_attributes);

    let (_b, app, middle) = ::app();
    let mut req = ::new_req_with_badges(app, krate.clone(), "1.0.0", badges);

    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));

    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foobadger");
    assert_eq!(json.krate.max_version, "1.0.0");

    let mut response = ok_resp!(
        middle.call(req.with_method(Method::Get)
                       .with_path("/api/v1/crates/foobadger")));

    let json: CrateResponse = ::json(&mut response);

    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0].badge_type, "travis-ci");
    assert_eq!(
        badges[0].attributes.get("repository").unwrap(),
        &String::from("rust-lang/crates.io")
    );
}

#[test]
fn ignored_badges() {
    let krate = ::krate("foo_ignored_badge");
    let mut badges = HashMap::new();

    // Known badge type, missing required repository attribute
    let mut badge_attributes = HashMap::new();
    badge_attributes.insert(
        String::from("branch"),
        String::from("master")
    );
    badges.insert(String::from("travis-ci"), badge_attributes);

    // Unknown badge type
    let mut unknown_badge_attributes = HashMap::new();
    unknown_badge_attributes.insert(
        String::from("repository"),
        String::from("rust-lang/rust")
    );
    badges.insert(String::from("not-a-badge"), unknown_badge_attributes);

    let (_b, app, middle) = ::app();
    let mut req = ::new_req_with_badges(app, krate.clone(), "1.0.0", badges);

    ::mock_user(&mut req, ::user("foo"));
    let mut response = ok_resp!(middle.call(&mut req));

    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_ignored_badge");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_badges.len(), 2);
    assert!(json.warnings.invalid_badges.contains(&"travis-ci".to_string()));
    assert!(json.warnings.invalid_badges.contains(&"not-a-badge".to_string()));

    let mut response = ok_resp!(
        middle.call(req.with_method(Method::Get)
                       .with_path("/api/v1/crates/foo_ignored_badge")));

    let json: CrateResponse = ::json(&mut response);

    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 0);
}

#[test]
fn reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let v100 = semver::Version::parse("1.0.0").unwrap();
    let v110 = semver::Version::parse("1.1.0").unwrap();
    let mut req = ::req(app, Method::Get,
                        "/api/v1/crates/c1/reverse_dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (c1, _) = ::mock_crate_vers(&mut req, ::krate("c1"), &v100);
    let (_, c2v1) = ::mock_crate_vers(&mut req, ::krate("c2"), &v100);
    let (_, c2v2) = ::mock_crate_vers(&mut req, ::krate("c2"), &v110);

    ::mock_dep(&mut req, &c2v1, &c1, None);
    ::mock_dep(&mut req, &c2v2, &c1, None);
    ::mock_dep(&mut req, &c2v2, &c1, Some("foo"));

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c2");

    // c1 has no dependent crates.
    req.with_path("/api/v1/crates/c2/reverse_dependencies");
    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn reverse_dependencies_when_old_version_doesnt_depend_but_new_does() {
    let (_b, app, middle) = ::app();

    let v100 = semver::Version::parse("1.0.0").unwrap();
    let v110 = semver::Version::parse("1.1.0").unwrap();
    let v200 = semver::Version::parse("2.0.0").unwrap();
    let mut req = ::req(app, Method::Get,
                        "/api/v1/crates/c1/reverse_dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (c1, _) = ::mock_crate_vers(&mut req, ::krate("c1"), &v110);
    let _ = ::mock_crate_vers(&mut req, ::krate("c2"), &v100);
    let (_, c2v2) = ::mock_crate_vers(&mut req, ::krate("c2"), &v200);

    ::mock_dep(&mut req, &c2v2, &c1, None);

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c2");
}

#[test]
fn reverse_dependencies_when_old_version_depended_but_new_doesnt() {
    let (_b, app, middle) = ::app();

    let v100 = semver::Version::parse("1.0.0").unwrap();
    let v200 = semver::Version::parse("2.0.0").unwrap();
    let mut req = ::req(app, Method::Get,
                        "/api/v1/crates/c1/reverse_dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (c1, _) = ::mock_crate_vers(&mut req, ::krate("c1"), &v100);
    let (_, c2v1) = ::mock_crate_vers(&mut req, ::krate("c2"), &v100);
    let _ = ::mock_crate_vers(&mut req, ::krate("c2"), &v200);

    ::mock_dep(&mut req, &c2v1, &c1, None);

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn prerelease_versions_not_included_in_reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let v100 = semver::Version::parse("1.0.0").unwrap();
    let v110_pre = semver::Version::parse("1.1.0-pre").unwrap();
    let mut req = ::req(app, Method::Get,
                        "/api/v1/crates/c1/reverse_dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (c1, _) = ::mock_crate_vers(&mut req, ::krate("c1"), &v100);
    let _ = ::mock_crate_vers(&mut req, ::krate("c2"), &v110_pre);
    let (_, c3v1) = ::mock_crate_vers(&mut req, ::krate("c3"), &v100);
    let _ = ::mock_crate_vers(&mut req, ::krate("c3"), &v110_pre);

    ::mock_dep(&mut req, &c3v1, &c1, None);

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c3");
}

#[test]
fn yanked_versions_not_included_in_reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let v100 = semver::Version::parse("1.0.0").unwrap();
    let v200 = semver::Version::parse("2.0.0").unwrap();
    let mut req = ::req(app, Method::Get,
                        "/api/v1/crates/c1/reverse_dependencies");
    ::mock_user(&mut req, ::user("foo"));
    let (c1, _) = ::mock_crate_vers(&mut req, ::krate("c1"), &v100);
    let _ = ::mock_crate_vers(&mut req, ::krate("c2"), &v100);
    let (_, c2v2) = ::mock_crate_vers(&mut req, ::krate("c2"), &v200);

    ::mock_dep(&mut req, &c2v2, &c1, None);

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c2");

    t!(c2v2.yank(req.tx().unwrap(), true));

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn author_license_and_description_required() {
    let (_b, app, middle) = ::app();
    ::user("foo");

    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    let mut new_crate = new_crate("foo_metadata");
    new_crate.license = None;
    new_crate.description = None;
    new_crate.authors = Vec::new();
    req.with_body(&::new_crate_to_body(&new_crate, &[]));
    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("author") &&
            json.errors[0].detail.contains("description") &&
            json.errors[0].detail.contains("license"),
            "{:?}", json.errors);

    new_crate.license = Some("MIT".to_string());
    new_crate.authors.push("".to_string());
    req.with_body(&::new_crate_to_body(&new_crate, &[]));
    let json = bad_resp!(middle.call(&mut req));
    assert!(json.errors[0].detail.contains("author") &&
            json.errors[0].detail.contains("description") &&
            !json.errors[0].detail.contains("license"),
            "{:?}", json.errors);

    new_crate.license = None;
    new_crate.license_file = Some("foo".to_string());
    new_crate.authors.push("foo".to_string());
    req.with_body(&::new_crate_to_body(&new_crate, &[]));
    let json = bad_resp!(middle.call(&mut req));
    assert!(!json.errors[0].detail.contains("author") &&
            json.errors[0].detail.contains("description") &&
            !json.errors[0].detail.contains("license"),
            "{:?}", json.errors);
}

