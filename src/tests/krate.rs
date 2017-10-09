extern crate diesel;

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::{self, File};

use chrono::Utc;
use conduit::{Handler, Method};
use diesel::update;
use self::diesel::prelude::*;
use git2;
use semver;
use serde_json;

use cargo_registry::dependency::EncodableDependency;
use cargo_registry::download::EncodableVersionDownload;
use cargo_registry::git;
use cargo_registry::keyword::EncodableKeyword;
use cargo_registry::krate::{Crate, EncodableCrate, MAX_NAME_LENGTH};

use cargo_registry::token::ApiToken;
use cargo_registry::schema::{crates, metadata, versions};

use cargo_registry::upload as u;
use cargo_registry::version::EncodableVersion;
use cargo_registry::category::{Category, EncodableCategory};

use {CrateList, CrateMeta, GoodCrate};

#[derive(Deserialize)]
struct VersionsList {
    versions: Vec<EncodableVersion>,
}
#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")] krate: EncodableCrate,
    versions: Vec<EncodableVersion>,
    keywords: Vec<EncodableKeyword>,
}
#[derive(Deserialize)]
struct Deps {
    dependencies: Vec<EncodableDependency>,
}
#[derive(Deserialize)]
struct RevDeps {
    dependencies: Vec<EncodableDependency>,
    versions: Vec<EncodableVersion>,
    meta: CrateMeta,
}
#[derive(Deserialize)]
struct Downloads {
    version_downloads: Vec<EncodableVersionDownload>,
}

#[derive(Deserialize)]
struct SummaryResponse {
    num_downloads: i64,
    num_crates: i64,
    new_crates: Vec<EncodableCrate>,
    most_downloaded: Vec<EncodableCrate>,
    most_recently_downloaded: Vec<EncodableCrate>,
    just_updated: Vec<EncodableCrate>,
    popular_keywords: Vec<EncodableKeyword>,
    popular_categories: Vec<EncodableCategory>,
}

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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);

    let krate = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("fooindex", u.id).expect_build(&conn)
    };

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

    let u;
    let krate;
    let krate2;
    {
        let conn = app.diesel_database.get().unwrap();
        u = ::new_user("foo").create_or_update(&conn).unwrap();

        krate = ::CrateBuilder::new("foo_index_queries", u.id)
            .readme("readme")
            .description("description")
            .keyword("kw1")
            .expect_build(&conn);

        krate2 = ::CrateBuilder::new("BAR_INDEX_QUERIES", u.id)
            .keyword("KW1")
            .expect_build(&conn);

        ::CrateBuilder::new("foo", u.id)
            .keyword("kw3")
            .expect_build(&conn);
    }

    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(req.with_query("q=baz")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 0);

    // All of these fields should be indexed/searched by the queries
    let mut response = ok_resp!(middle.call(req.with_query("q=foo")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 2);
    let mut response = ok_resp!(middle.call(req.with_query("q=kw1")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 2);
    let mut response = ok_resp!(middle.call(req.with_query("q=readme")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("q=description")));
    assert_eq!(::json::<CrateList>(&mut response).meta.total, 1);

    let query = format!("user_id={}", u.id);
    let mut response = ok_resp!(middle.call(req.with_query(&query)));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 3);
    let mut response = ok_resp!(middle.call(req.with_query("user_id=0")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);

    let mut response = ok_resp!(middle.call(req.with_query("letter=F")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 2);
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

    let mut response = ok_resp!(middle.call(req.with_query("q=foo&keyword=kw1")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 1);
    let mut response = ok_resp!(middle.call(req.with_query("q=foo2&keyword=kw1")));
    assert_eq!(::json::<CrateList>(&mut response).crates.len(), 0);

    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("Category 1", "cat1")
            .create_or_update(&conn)
            .unwrap();
        ::new_category("Category 1::Ba'r", "cat1::bar")
            .create_or_update(&conn)
            .unwrap();
        Category::update_crate(&conn, &krate, &["cat1"]).unwrap();
        Category::update_crate(&conn, &krate2, &["cat1::bar"]).unwrap();
    }
    let mut response = ok_resp!(middle.call(req.with_query("category=cat1")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 2);
    assert_eq!(cl.meta.total, 2);
    let mut response = ok_resp!(middle.call(req.with_query("category=cat1::bar")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);
    let mut response = ok_resp!(middle.call(req.with_query("keyword=cat2")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 0);
    assert_eq!(cl.meta.total, 0);

    let mut response = ok_resp!(middle.call(req.with_query("q=readme&category=cat1")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);

    let mut response = ok_resp!(middle.call(req.with_query("keyword=kw1&category=cat1")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 2);
    assert_eq!(cl.meta.total, 2);

    let mut response = ok_resp!(middle.call(req.with_query("keyword=kw3&category=cat1")));
    let cl = ::json::<CrateList>(&mut response);
    assert_eq!(cl.crates.len(), 0);
    assert_eq!(cl.meta.total, 0);
}

#[test]
fn search_includes_crates_where_name_is_stopword() {
    let (_b, app, middle) = ::app();
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();

        ::CrateBuilder::new("which", u.id).expect_build(&conn);
        ::CrateBuilder::new("should_be_excluded", u.id)
            .readme("crate which does things")
            .expect_build(&conn);
    }
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(req.with_query("q=which")));
    let json = ::json::<CrateList>(&mut response);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
}

#[test]
fn exact_match_first_on_queries() {
    let (_b, app, middle) = ::app();

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();

        ::CrateBuilder::new("foo_exact", user.id)
            .description("bar_exact baz_exact")
            .expect_build(&conn);

        ::CrateBuilder::new("bar-exact", user.id)
            .description("foo_exact baz_exact foo-exact baz_exact")
            .expect_build(&conn);

        ::CrateBuilder::new("baz_exact", user.id)
            .description("foo-exact bar_exact foo-exact bar_exact foo_exact bar_exact")
            .expect_build(&conn);

        ::CrateBuilder::new("other_exact", user.id)
            .description("other_exact")
            .expect_build(&conn);
    }

    let mut req = ::req(app, Method::Get, "/api/v1/crates");

    let mut response = ok_resp!(middle.call(req.with_query("q=foo-exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "foo_exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "bar-exact");

    let mut response = ok_resp!(middle.call(req.with_query("q=bar_exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar-exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "foo_exact");

    let mut response = ok_resp!(middle.call(req.with_query("q=baz_exact")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_exact");
    assert_eq!(json.crates[1].name, "bar-exact");
    assert_eq!(json.crates[2].name, "foo_exact");
}

#[test]
fn exact_match_on_queries_with_sort() {
    let (_b, app, middle) = ::app();

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();

        ::CrateBuilder::new("foo_sort", user.id)
            .description("bar_sort baz_sort const")
            .downloads(50)
            .recent_downloads(50)
            .expect_build(&conn);

        ::CrateBuilder::new("bar_sort", user.id)
            .description("foo_sort baz_sort foo_sort baz_sort const")
            .downloads(3333)
            .recent_downloads(0)
            .expect_build(&conn);

        ::CrateBuilder::new("baz_sort", user.id)
            .description("foo_sort bar_sort foo_sort bar_sort foo_sort bar_sort const")
            .downloads(100000)
            .recent_downloads(10)
            .expect_build(&conn);

        ::CrateBuilder::new("other_sort", user.id)
            .description("other_sort const")
            .downloads(999999)
            .expect_build(&conn);
    }

    // Sort by downloads
    let mut req = ::req(app, Method::Get, "/api/v1/crates");
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

    // Sort by recent-downloads
    let mut response = ok_resp!(middle.call(req.with_query("q=bar_sort&sort=recent-downloads"),));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "foo_sort");
    assert_eq!(json.crates[2].name, "baz_sort");

    // Test for bug with showing null results first when sorting
    // by descending
    // This has nothing to do with querying for exact match I'm sorry
    let mut response = ok_resp!(middle.call(req.with_query("sort=recent-downloads")));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "other_sort");
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates/foo_show");
    let krate;
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        krate = ::CrateBuilder::new("foo_show", user.id)
            .description("description")
            .documentation("https://example.com")
            .homepage("http://example.com")
            .version(::VersionBuilder::new("1.0.0"))
            .version(::VersionBuilder::new("0.5.0"))
            .version(::VersionBuilder::new("0.5.1"))
            .keyword("kw1")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.id, krate.name);
    assert_eq!(json.krate.description, krate.description);
    assert_eq!(json.krate.homepage, krate.homepage);
    assert_eq!(json.krate.documentation, krate.documentation);
    assert_eq!(json.krate.keywords, Some(vec!["kw1".into()]));
    assert_eq!(json.krate.recent_downloads, Some(10));
    let versions = json.krate.versions.as_ref().unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(json.versions.len(), 3);

    assert_eq!(json.versions[0].id, versions[0]);
    assert_eq!(json.versions[0].krate, json.krate.id);
    assert_eq!(json.versions[0].num, "1.0.0");
    let suffix = "/api/v1/crates/foo_show/1.0.0/download";
    assert!(
        json.versions[0].dl_path.ends_with(suffix),
        "bad suffix {}",
        json.versions[0].dl_path
    );
    assert_eq!(1, json.keywords.len());
    assert_eq!("kw1", json.keywords[0].id);

    assert_eq!(json.versions[1].num, "0.5.1");
    assert_eq!(json.versions[2].num, "0.5.0");
}

#[test]
fn versions() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_versions/versions",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_versions", u.id)
            .version("0.5.1")
            .version("1.0.0")
            .version("0.5.0")
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionsList = ::json(&mut response);

    assert_eq!(json.versions.len(), 3);
    assert_eq!(json.versions[0].num, "1.0.0");
    assert_eq!(json.versions[1].num, "0.5.1");
    assert_eq!(json.versions[2].num, "0.5.0");
}

#[test]
fn uploading_new_version_touches_crate() {
    use diesel::expression::dsl::*;

    let (_b, app, middle) = ::app();

    let mut upload_req = ::new_req(app.clone(), "foo_versions_updated_at", "1.0.0");
    let u = ::sign_in(&mut upload_req, &app);
    ok_resp!(middle.call(&mut upload_req));

    {
        let conn = app.diesel_database.get().unwrap();
        diesel::update(crates::table)
            .set(crates::updated_at.eq(crates::updated_at - 1.hour()))
            .execute(&*conn)
            .unwrap();
    }

    let mut show_req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_versions_updated_at",
    );
    let mut response = ok_resp!(middle.call(&mut show_req));
    let json: CrateResponse = ::json(&mut response);
    let updated_at_before = json.krate.updated_at;

    let mut upload_req = ::new_req(app.clone(), "foo_versions_updated_at", "2.0.0");
    ::sign_in_as(&mut upload_req, &u);
    ok_resp!(middle.call(&mut upload_req));

    let mut response = ok_resp!(middle.call(&mut show_req));
    let json: CrateResponse = ::json(&mut response);
    let updated_at_after = json.krate.updated_at;

    assert_ne!(updated_at_before, updated_at_after);
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

    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    ::sign_in(&mut req, &app);
    ::logout(&mut req);
    req.header("Authorization", "bad");
    bad_resp!(middle.call(&mut req));
}

#[test]
fn new_bd_names() {
    fn bad_name(name: &str) {
        println!("testing: `{}`", name);
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app.clone(), name, "1.0.0");
        ::sign_in(&mut req, &app);
        let json = bad_resp!(middle.call(&mut req));
        assert!(
            json.errors[0]
                .detail
                .contains("expected a valid crate name",),
            "{:?}",
            json.errors
        );
    }

    bad_name("");
    bad_name("foo bar");
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1));
}

#[test]
fn new_krate() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_new", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_with_token() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_new", "1.0.0");

    {
        let conn = t!(app.diesel_database.get());
        let user = t!(::new_user("foo").create_or_update(&conn));
        let token = t!(ApiToken::insert(&conn, user.id, "bar"));
        req.header("Authorization", &token.token);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_with_reserved_name() {
    fn test_bad_name(name: &str) {
        let (_b, app, middle) = ::app();
        let mut req = ::new_req(app.clone(), name, "1.0.0");
        ::sign_in(&mut req, &app);
        let json = bad_resp!(middle.call(&mut req));
        assert!(
            json.errors[0]
                .detail
                .contains("cannot upload a crate with a reserved name",)
        );
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
    let mut req = ::new_req(app.clone(), "foo_weird", "0.0.0-pre");
    ::sign_in(&mut req, &app);
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
    let mut req = ::new_req_full(app.clone(), ::krate("new_dep"), "1.0.0", vec![dep]);
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_dep", user.id).expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let path = ::git::checkout().join("ne/w_/new_dep");
    assert!(path.exists());
    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    let p: git::Crate = serde_json::from_str(&contents).unwrap();
    assert_eq!(p.name, "new_dep");
    assert_eq!(p.vers, "1.0.0");
    assert_eq!(p.deps.len(), 1);
    assert_eq!(p.deps[0].name, "foo_dep");
}

#[test]
fn new_krate_non_canon_crate_name_dependencies() {
    let (_b, app, middle) = ::app();
    let deps = vec![
        u::CrateDependency {
            name: u::CrateName("foo-dep".to_string()),
            optional: false,
            default_features: true,
            features: Vec::new(),
            version_req: u::CrateVersionReq(semver::VersionReq::parse(">= 0").unwrap()),
            target: None,
            kind: None,
        },
    ];
    let mut req = ::new_req_full(app.clone(), ::krate("new_dep"), "1.0.0", deps);
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo-dep", user.id).expect_build(&conn);
    }

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
    let mut req = ::new_req_full(app.clone(), ::krate("new_wild"), "1.0.0", vec![dep]);
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_wild", user.id).expect_build(&conn);
    }
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("dependency constraints"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_twice() {
    let (_b, app, middle) = ::app();
    let mut krate = ::krate("foo_twice");
    krate.description = Some("description".to_string());
    let mut req = ::new_req_full(app.clone(), krate.clone(), "2.0.0", Vec::new());
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_twice", user.id).expect_build(&conn);
    }
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.description, krate.description);
}

#[test]
fn new_krate_wrong_user() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app.clone(), "foo_wrong", "2.0.0");

    {
        // Create the 'foo' crate with one user
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_wrong", user.id).expect_build(&conn);

        // But log in another
        let user = ::new_user("bar").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0]
            .detail
            .contains("this crate exists but you don't seem to be an owner.",),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_bad_name() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foobar", "2.0.0");
    let user = ::sign_in(&mut req, &app);
    {
        let mut req = ::new_req(app.clone(), "snow☃", "2.0.0");
        ::sign_in_as(&mut req, &user);
        let json = bad_resp!(middle.call(&mut req));
        assert!(
            json.errors[0]
                .detail
                .contains("expected a valid crate name",),
            "{:?}",
            json.errors
        );
    }
    {
        let mut req = ::new_req(app.clone(), "áccênts", "2.0.0");
        ::sign_in_as(&mut req, &user);
        let json = bad_resp!(middle.call(&mut req));
        assert!(
            json.errors[0]
                .detail
                .contains("expected a valid crate name",),
            "{:?}",
            json.errors
        );
    }
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
    let mut req = ::new_req(app.clone(), "foo_big", "1.0.0");
    ::sign_in(&mut req, &app);
    let files = [("foo_big-1.0.0/big", &[b'a'; 2000] as &[_])];
    let body = ::new_crate_to_body(&new_crate("foo_big"), &files);
    bad_resp!(middle.call(req.with_body(&body)));
}

#[test]
fn new_krate_too_big_but_whitelisted() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_whitelist", "1.1.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_whitelist", user.id)
            .max_upload_size(2_000_000)
            .expect_build(&conn);
    }
    let files = [("foo_whitelist-1.1.0/big", &[b'a'; 2000] as &[_])];
    let body = ::new_crate_to_body(&new_crate("foo_whitelist"), &files);
    let mut response = ok_resp!(middle.call(req.with_body(&body)));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn new_krate_wrong_files() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    ::sign_in(&mut req, &app);
    let data: &[u8] = &[1];
    let files = [("foo-1.0.0/a", data), ("bar-1.0.0/a", data)];
    let body = ::new_crate_to_body(&new_crate("foo"), &files);
    bad_resp!(middle.call(req.with_body(&body)));
}

#[test]
fn new_krate_duplicate_version() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_dupe", "1.0.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);

        ::CrateBuilder::new("foo_dupe", user.id)
            .version(::VersionBuilder::new("1.0.0"))
            .expect_build(&conn);
    }
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("already uploaded"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_similar", "1.1.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &u);
        ::CrateBuilder::new("Foo_similar", u.id).expect_build(&conn);
    }
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name_hyphen() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo-bar-hyphen", "1.1.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &u);
        ::CrateBuilder::new("foo_bar_hyphen", u.id).expect_build(&conn);
    }
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name_underscore() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_bar_underscore", "1.1.0");
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &u);
        ::CrateBuilder::new("foo-bar-underscore", u.id).expect_build(&conn);
    }
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_git_upload() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "fgt", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let path = ::git::checkout().join("3/f/fgt");
    assert!(path.exists());
    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    let p: git::Crate = serde_json::from_str(&contents).unwrap();
    assert_eq!(p.name, "fgt");
    assert_eq!(p.vers, "1.0.0");
    assert!(p.deps.is_empty());
    assert_eq!(
        p.cksum,
        "acb5604b126ac894c1eb11c4575bf2072fea61232a888e453770c79d7ed56419"
    );
}

#[test]
fn new_krate_git_upload_appends() {
    let (_b, app, middle) = ::app();
    let path = ::git::checkout().join("3/f/fpp");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    File::create(&path)
        .unwrap()
        .write_all(
            br#"{"name":"FPP","vers":"0.0.1","deps":[],"features":{},"cksum":"3j3"}
"#,
        )
        .unwrap();

    let mut req = ::new_req(app.clone(), "FPP", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    let mut lines = contents.lines();
    let p1: git::Crate = serde_json::from_str(lines.next().unwrap().trim()).unwrap();
    let p2: git::Crate = serde_json::from_str(lines.next().unwrap().trim()).unwrap();
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
        repo.commit(Some("HEAD"), &sig, &sig, "empty commit", &tree, &[&parent])
            .unwrap();
    }

    let mut req = ::new_req(app.clone(), "foo_conflicts", "1.0.0");
    ::sign_in(&mut req, &app);
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
    let mut req = ::new_req_full(app.clone(), ::krate("foo_missing"), "1.0.0", vec![dep]);
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    let json = ::json::<::Bad>(&mut response);
    assert!(
        json.errors[0]
            .detail
            .contains("no known crate named `bar_missing`",)
    );
}

#[test]
fn new_krate_with_readme() {
    let (_b, app, middle) = ::app();
    let mut krate = ::krate("foo_readme");
    krate.readme = Some("".to_owned());
    let mut req = ::new_req_full(app.clone(), krate, "1.0.0", vec![]);
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn summary_doesnt_die() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/summary");
    ok_resp!(middle.call(&mut req));
}

#[test]
fn summary_new_crates() {
    let (_b, app, middle) = ::app();
    let u;
    let krate;
    let krate2;
    let krate3;
    {
        let conn = app.diesel_database.get().unwrap();
        u = ::new_user("foo").create_or_update(&conn).unwrap();

        krate = ::CrateBuilder::new("some_downloads", u.id)
            .version(::VersionBuilder::new("0.1.0"))
            .description("description")
            .keyword("popular")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(&conn);

        krate2 = ::CrateBuilder::new("most_recent_downloads", u.id)
            .version(::VersionBuilder::new("0.2.0"))
            .keyword("popular")
            .downloads(5000)
            .recent_downloads(50)
            .expect_build(&conn);

        krate3 = ::CrateBuilder::new("just_updated", u.id)
            .version(::VersionBuilder::new("0.1.0"))
            .expect_build(&conn);

        ::CrateBuilder::new("with_downloads", u.id)
            .version(::VersionBuilder::new("0.3.0"))
            .keyword("popular")
            .downloads(1000)
            .expect_build(&conn);

        ::new_category("Category 1", "cat1")
            .create_or_update(&conn)
            .unwrap();
        Category::update_crate(&conn, &krate, &["cat1"]).unwrap();
        Category::update_crate(&conn, &krate2, &["cat1"]).unwrap();

        // set total_downloads global value for `num_downloads` prop
        update(metadata::table)
            .set(metadata::total_downloads.eq(6000))
            .execute(&*conn)
            .unwrap();

        // update 'just_updated' krate. Others won't appear because updated_at == created_at.
        let updated = Utc::now().naive_utc();
        update(&krate3)
            .set(crates::updated_at.eq(updated))
            .execute(&*conn)
            .unwrap();
    }

    let mut req = ::req(app.clone(), Method::Get, "/api/v1/summary");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: SummaryResponse = ::json(&mut response);

    assert_eq!(json.num_crates, 4);
    assert_eq!(json.num_downloads, 6000);
    assert_eq!(json.most_downloaded[0].name, "most_recent_downloads");
    assert_eq!(
        json.most_recently_downloaded[0].name,
        "most_recent_downloads"
    );
    assert_eq!(json.popular_keywords[0].keyword, "popular");
    assert_eq!(json.popular_categories[0].category, "Category 1");
    assert_eq!(json.just_updated.len(), 1);
    assert_eq!(json.just_updated[0].name, "just_updated");
    assert_eq!(json.new_crates.len(), 4);
}

#[test]
fn download() {
    use chrono::{Duration, Utc};
    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_download/1.0.0/download",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_download", user.id)
            .version(::VersionBuilder::new("1.0.0"))
            .expect_build(&conn);
    }
    let resp = t_resp!(middle.call(&mut req));
    assert_eq!(resp.status.0, 302);

    req.with_path("/api/v1/crates/foo_download/1.0.0/downloads");
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);
    req.with_path("/api/v1/crates/foo_download/downloads");
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
    req.with_path("/api/v1/crates/FOO_DOWNLOAD/downloads");
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);

    let yesterday = Utc::today() + Duration::days(-1);
    req.with_path("/api/v1/crates/FOO_DOWNLOAD/1.0.0/downloads");
    req.with_query(&format!("before_date={}", yesterday.format("%F")));
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 0);
    req.with_path("/api/v1/crates/FOO_DOWNLOAD/downloads");
    req.with_query(&format!("before_date={}", yesterday.format("%F")));
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    // crate/downloads always returns the last 90 days and ignores date params
    assert_eq!(downloads.version_downloads.len(), 1);

    let tomorrow = Utc::today() + Duration::days(1);
    req.with_path("/api/v1/crates/FOO_DOWNLOAD/1.0.0/downloads");
    req.with_query(&format!("before_date={}", tomorrow.format("%F")));
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);
    req.with_path("/api/v1/crates/FOO_DOWNLOAD/downloads");
    req.with_query(&format!("before_date={}", tomorrow.format("%F")));
    let mut resp = ok_resp!(middle.call(&mut req));
    let downloads = ::json::<Downloads>(&mut resp);
    assert_eq!(downloads.version_downloads.len(), 1);
}

#[test]
fn download_bad() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_bad/0.1.0/download",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_bad", user.id).expect_build(&conn);
    }
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(404, response.status.0)
}

#[test]
fn dependencies() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_deps/1.0.0/dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("foo_deps", user.id).expect_build(&conn);
        let v = ::new_version(c1.id, "1.0.0").save(&conn, &[]).unwrap();
        let c2 = ::CrateBuilder::new("bar_deps", user.id).expect_build(&conn);
        ::new_dependency(&conn, &v, &c2);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<Deps>(&mut response);
    assert_eq!(deps.dependencies[0].crate_id, "bar_deps");

    req.with_path("/api/v1/crates/foo_deps/1.0.2/dependencies");
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<::Bad>(&mut response);
}

#[test]
fn diesel_not_found_results_in_404() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_following/following",
    );

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let response = middle.call(&mut req).unwrap();
    assert_eq!((404, "Not Found"), response.status);
}

#[test]
fn following() {
    #[derive(Deserialize)]
    struct F {
        following: bool,
    }
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/foo_following/following",
    );

    let user;
    {
        let conn = app.diesel_database.get().unwrap();
        user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        ::CrateBuilder::new("foo_following", user.id).expect_build(&conn);
    }

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
        .with_method(Method::Get)
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
fn yank() {
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }
    #[derive(Deserialize)]
    struct V {
        version: EncodableVersion,
    }
    let (_b, app, middle) = ::app();
    let path = ::git::checkout().join("3/f/fyk");

    // Upload a new crate, putting it in the git index
    let mut req = ::new_req(app.clone(), "fyk", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);
    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert!(contents.contains("\"yanked\":false"));

    // make sure it's not yanked
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk/1.0.0",),
        )
    );
    assert!(!::json::<V>(&mut r).version.yanked);

    // yank it
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Delete,)
                .with_path("/api/v1/crates/fyk/1.0.0/yank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert!(contents.contains("\"yanked\":true"));
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk/1.0.0",),
        )
    );
    assert!(::json::<V>(&mut r).version.yanked);

    // un-yank it
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Put,)
                .with_path("/api/v1/crates/fyk/1.0.0/unyank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut contents = String::new();
    File::open(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert!(contents.contains("\"yanked\":false"));
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk/1.0.0",),
        )
    );
    assert!(!::json::<V>(&mut r).version.yanked);
}

#[test]
fn yank_not_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::request_with_user_and_mock_crate(&app, ::new_user("bar"), "foo_not");
    ::sign_in(&mut req, &app);
    req.with_method(Method::Delete)
        .with_path("/api/v1/crates/foo_not/1.0.0/yank");
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<::Bad>(&mut response);
}

#[test]
fn yank_max_version() {
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }
    let (_b, app, middle) = ::app();

    // Upload a new crate
    let mut req = ::new_req(app.clone(), "fyk_max", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));

    // double check the max version
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let body = ::new_req_body_version_2(::krate("fyk_max"));
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_method(Method::Put)
                .with_body(&body),
        )
    );
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Delete,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/yank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Put,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/unyank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Delete,)
                .with_path("/api/v1/crates/fyk_max/2.0.0/yank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Delete,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/yank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Put,)
                .with_path("/api/v1/crates/fyk_max/2.0.0/unyank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Put,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/unyank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_yank_max_version() {
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }
    let (_b, app, middle) = ::app();

    // Upload a new crate
    let mut req = ::new_req(app.clone(), "fyk_max", "1.0.0");
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));

    // double check the max version
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Delete,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/yank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let body = ::new_req_body_version_2(::krate("fyk_max"));
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/new")
                .with_method(Method::Put)
                .with_body(&body),
        )
    );
    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    let mut r = ok_resp!(
        middle.call(
            req.with_method(Method::Put,)
                .with_path("/api/v1/crates/fyk_max/1.0.0/unyank",),
        )
    );
    assert!(::json::<O>(&mut r).ok);
    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/fyk_max",),
        )
    );
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn bad_keywords() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foobar", "1.0.0");
    let user = ::sign_in(&mut req, &app);
    {
        let krate = ::krate("foo_bad_key");
        let kws = vec!["super-long-keyword-name-oh-no".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::sign_in_as(&mut req, &user);
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key2");
        let kws = vec!["?@?%".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::sign_in_as(&mut req, &user);
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key_3");
        let kws = vec!["?@?%".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::sign_in_as(&mut req, &user);
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
    {
        let krate = ::krate("foo_bad_key4");
        let kws = vec!["áccênts".into()];
        let mut req = ::new_req_with_keywords(app.clone(), krate, "1.0.0", kws);
        ::sign_in_as(&mut req, &user);
        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<::Bad>(&mut response);
    }
}

#[test]
fn good_categories() {
    let (_b, app, middle) = ::app();
    let krate = ::krate("foo_good_cat");
    let cats = vec!["cat1".into()];
    let mut req = ::new_req_with_categories(app.clone(), krate, "1.0.0", cats);
    ::sign_in(&mut req, &app);
    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("Category 1", "cat1")
            .create_or_update(&conn)
            .unwrap();
    }
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
    let mut req = ::new_req_with_categories(app.clone(), krate, "1.0.0", cats);
    ::sign_in(&mut req, &app);
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
        String::from("rust-lang/crates.io"),
    );
    badges.insert(String::from("travis-ci"), badge_attributes);

    let (_b, app, middle) = ::app();
    let mut req = ::new_req_with_badges(app.clone(), krate.clone(), "1.0.0", badges);
    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));

    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foobadger");
    assert_eq!(json.krate.max_version, "1.0.0");

    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/foobadger",),
        )
    );

    let json: CrateResponse = ::json(&mut response);

    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0].badge_type, "travis-ci");
    assert_eq!(
        badges[0].attributes.get("repository").unwrap(),
        &Some(String::from("rust-lang/crates.io"))
    );
}

#[test]
fn ignored_badges() {
    let krate = ::krate("foo_ignored_badge");
    let mut badges = HashMap::new();

    // Known badge type, missing required repository attribute
    let mut badge_attributes = HashMap::new();
    badge_attributes.insert(String::from("branch"), String::from("master"));
    badges.insert(String::from("travis-ci"), badge_attributes);

    // Unknown badge type
    let mut unknown_badge_attributes = HashMap::new();
    unknown_badge_attributes.insert(String::from("repository"), String::from("rust-lang/rust"));
    badges.insert(String::from("not-a-badge"), unknown_badge_attributes);

    let (_b, app, middle) = ::app();
    let mut req = ::new_req_with_badges(app.clone(), krate.clone(), "1.0.0", badges);

    ::sign_in(&mut req, &app);
    let mut response = ok_resp!(middle.call(&mut req));

    let json: GoodCrate = ::json(&mut response);
    assert_eq!(json.krate.name, "foo_ignored_badge");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_badges.len(), 2);
    assert!(
        json.warnings
            .invalid_badges
            .contains(&"travis-ci".to_string(),)
    );
    assert!(
        json.warnings
            .invalid_badges
            .contains(&"not-a-badge".to_string(),)
    );

    let mut response = ok_resp!(
        middle.call(
            req.with_method(Method::Get,)
                .with_path("/api/v1/crates/foo_ignored_badge",),
        )
    );

    let json: CrateResponse = ::json(&mut response);

    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 0);
}

#[test]
fn reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/c1/reverse_dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("c1", u.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::CrateBuilder::new("c2", u.id)
            .version(::VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version(
                ::VersionBuilder::new("1.1.0")
                    .dependency(&c1, None)
                    .dependency(&c1, Some("foo")),
            )
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
    assert_eq!(deps.versions.len(), 1);
    assert_eq!(deps.versions[0].krate, "c2");
    assert_eq!(deps.versions[0].num, "1.1.0");

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

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/c1/reverse_dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("c1", u.id)
            .version("1.1.0")
            .expect_build(&conn);
        ::CrateBuilder::new("c2", u.id)
            .version("1.0.0")
            .version(::VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn reverse_dependencies_when_old_version_depended_but_new_doesnt() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/c1/reverse_dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("c1", u.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::CrateBuilder::new("c2", u.id)
            .version(::VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("2.0.0")
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn prerelease_versions_not_included_in_reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/c1/reverse_dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("c1", u.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::CrateBuilder::new("c2", u.id)
            .version("1.1.0-pre")
            .expect_build(&conn);
        ::CrateBuilder::new("c3", u.id)
            .version(::VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("1.1.0-pre")
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn yanked_versions_not_included_in_reverse_dependencies() {
    let (_b, app, middle) = ::app();

    let mut req = ::req(
        app.clone(),
        Method::Get,
        "/api/v1/crates/c1/reverse_dependencies",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c1 = ::CrateBuilder::new("c1", u.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::CrateBuilder::new("c2", u.id)
            .version("1.0.0")
            .version(::VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(&mut req));
    let deps = ::json::<RevDeps>(&mut response);
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");

    // TODO: have this test call `version.yank()` once the yank method is converted to diesel
    diesel::update(versions::table.filter(versions::num.eq("2.0.0")))
        .set(versions::yanked.eq(true))
        .execute(&*app.diesel_database.get().unwrap())
        .unwrap();

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
    assert!(
        json.errors[0].detail.contains("author") && json.errors[0].detail.contains("description")
            && json.errors[0].detail.contains("license"),
        "{:?}",
        json.errors
    );

    new_crate.license = Some("MIT".to_string());
    new_crate.authors.push("".to_string());
    req.with_body(&::new_crate_to_body(&new_crate, &[]));
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        json.errors[0].detail.contains("author") && json.errors[0].detail.contains("description")
            && !json.errors[0].detail.contains("license"),
        "{:?}",
        json.errors
    );

    new_crate.license = None;
    new_crate.license_file = Some("foo".to_string());
    new_crate.authors.push("foo".to_string());
    req.with_body(&::new_crate_to_body(&new_crate, &[]));
    let json = bad_resp!(middle.call(&mut req));
    assert!(
        !json.errors[0].detail.contains("author") && json.errors[0].detail.contains("description")
            && !json.errors[0].detail.contains("license"),
        "{:?}",
        json.errors
    );
}

/*  Given two crates, one with downloads less than 90 days ago, the
    other with all downloads greater than 90 days ago, check that
    the order returned is by recent downloads, descending. Check
    also that recent download counts are returned in recent_downloads,
    and total downloads counts are returned in downloads, and that
    these numbers do not overlap.
*/
#[test]
fn test_recent_download_count() {
    let (_b, app, middle) = ::app();

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("Oskar").create_or_update(&conn).unwrap();

        // More than 90 days ago
        ::CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .downloads(10)
            .recent_downloads(0)
            .expect_build(&conn);

        ::CrateBuilder::new("sweet_potato_snack", user.id)
            .description("For when better than usual")
            .downloads(5)
            .recent_downloads(2)
            .expect_build(&conn);
    }

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(req.with_query("sort=recent-downloads")));
    let json: CrateList = ::json(&mut response);

    assert_eq!(json.meta.total, 2);

    assert_eq!(json.crates[0].name, "sweet_potato_snack");
    assert_eq!(json.crates[1].name, "green_ball");

    assert_eq!(json.crates[0].recent_downloads, Some(2));
    assert_eq!(json.crates[0].downloads, 5);

    assert_eq!(json.crates[1].recent_downloads, Some(0));
    assert_eq!(json.crates[1].downloads, 10);
}

/*  Given one crate with zero downloads, check that the crate
    still shows up in index results, but that it displays 0
    for both recent downloads and downloads.
 */
#[test]
fn test_zero_downloads() {
    let (_b, app, middle) = ::app();

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("Oskar").create_or_update(&conn).unwrap();

        // More than 90 days ago
        ::CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .downloads(0)
            .recent_downloads(0)
            .expect_build(&conn);
    }

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(req.with_query("sort=recent-downloads")));
    let json: CrateList = ::json(&mut response);

    assert_eq!(json.meta.total, 1);

    assert_eq!(json.crates[0].name, "green_ball");
    assert_eq!(json.crates[0].recent_downloads, Some(0));
    assert_eq!(json.crates[0].downloads, 0);
}

/*  Given two crates, one with more all-time downloads, the other with
    more downloads in the past 90 days, check that the index page for
    categories and keywords is sorted by recent downlaods by default.
*/
#[test]
fn test_default_sort_recent() {
    let (_b, app, middle) = ::app();

    let (green_crate, potato_crate) = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("Oskar").create_or_update(&conn).unwrap();

        // More than 90 days ago
        let green_crate = ::CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .keyword("dog")
            .downloads(10)
            .recent_downloads(10)
            .expect_build(&conn);

        let potato_crate = ::CrateBuilder::new("sweet_potato_snack", user.id)
            .description("For when better than usual")
            .keyword("dog")
            .downloads(20)
            .recent_downloads(0)
            .expect_build(&conn);

        (green_crate, potato_crate)
    };

    // test that index for keywords is sorted by recent_downloads
    // by default
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/crates");
    let mut response = ok_resp!(middle.call(req.with_query("keyword=dog")));
    let json: CrateList = ::json(&mut response);

    assert_eq!(json.meta.total, 2);

    assert_eq!(json.crates[0].name, "green_ball");
    assert_eq!(json.crates[1].name, "sweet_potato_snack");

    assert_eq!(json.crates[0].recent_downloads, Some(10));
    assert_eq!(json.crates[0].downloads, 10);

    assert_eq!(json.crates[1].recent_downloads, Some(0));
    assert_eq!(json.crates[1].downloads, 20);

    {
        let conn = app.diesel_database.get().unwrap();
        ::new_category("Animal", "animal")
            .create_or_update(&conn)
            .unwrap();
        Category::update_crate(&conn, &green_crate, &["animal"]).unwrap();
        Category::update_crate(&conn, &potato_crate, &["animal"]).unwrap();
    }

    // test that index for categories is sorted by recent_downloads
    // by default
    let mut response = ok_resp!(middle.call(req.with_query("category=animal")));
    let json: CrateList = ::json(&mut response);

    assert_eq!(json.meta.total, 2);

    assert_eq!(json.crates[0].name, "green_ball");
    assert_eq!(json.crates[1].name, "sweet_potato_snack");

    assert_eq!(json.crates[0].recent_downloads, Some(10));
    assert_eq!(json.crates[0].downloads, 10);

    assert_eq!(json.crates[1].recent_downloads, Some(0));
    assert_eq!(json.crates[1].downloads, 20);
}

#[test]
fn block_blacklisted_documentation_url() {
    let (_b, app, middle) = ::app();

    let _ = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_bad_doc_url", u.id)
            .documentation("http://rust-ci.org/foo/foo_bad_doc_url/doc/foo_bad_doc_url/")
            .expect_build(&conn)
    };

    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_bad_doc_url");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateResponse = ::json(&mut response);

    assert_eq!(json.krate.documentation, None);
}

// This is testing Cargo functionality! ! !
// specifically functions modify_owners and add_owners
// which call the `PUT /crates/:crate_id/owners` route
#[test]
fn test_cargo_invite_owners() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/");

    let new_user = {
        let conn = app.diesel_database.get().unwrap();
        let owner = ::new_user("avocado").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &owner);
        ::CrateBuilder::new("guacamole", owner.id).expect_build(&conn);
        ::new_user("cilantro").create_or_update(&conn).unwrap()
    };

    #[derive(Serialize)]
    struct OwnerReq {
        owners: Option<Vec<String>>,
    }
    #[derive(Deserialize, Debug)]
    struct OwnerResp {
        ok: bool,
        msg: String,
    }

    let body = serde_json::to_string(&OwnerReq {
        owners: Some(vec![new_user.gh_login]),
    });
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/guacamole/owners")
                .with_method(Method::Put)
                .with_body(body.unwrap().as_bytes()),
        )
    );

    let r = ::json::<OwnerResp>(&mut response);
    // this ok:true field is what old versions of Cargo
    // need - do not remove unless you're cool with
    // dropping support for old versions
    assert!(r.ok);
    // msg field is what is sent and used in updated
    // version of cargo
    assert_eq!(
        r.msg,
        "user cilantro has been invited to be an owner of crate guacamole"
    )
}

// #[test]
// fn new_crate_bad_tarball() {
//     let (_b, app, middle) = ::app();
//     let mut req = ::new_req(app.clone(), "foo_new", "1.0.0");
//     ::sign_in(&mut req, &app);
//     let mut response = ok_resp!(middle.call(&mut req));
//     let json: GoodCrate = ::json(&mut response);
//     assert_eq!(json.krate.name, "foo_new");
//     assert_eq!(json.krate.max_version, "1.0.0");
// }
