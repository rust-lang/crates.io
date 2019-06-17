use crate::{
    builders::{CrateBuilder, DependencyBuilder, PublishBuilder, VersionBuilder},
    new_category, new_dependency, new_user, CrateMeta, CrateResponse, GoodCrate, OkBool,
    RequestHelper, TestApp,
};
use cargo_registry::{
    models::{krate::MAX_NAME_LENGTH, Category, Crate},
    schema::{api_tokens, crates, emails, metadata, versions, versions_published_by},
    views::{
        EncodableCategory, EncodableCrate, EncodableDependency, EncodableKeyword, EncodableVersion,
        EncodableVersionDownload,
    },
};
use std::{
    collections::HashMap,
    io::{self, prelude::*},
    thread,
    time::Duration,
};

use chrono::Utc;
use diesel::{dsl::*, prelude::*, update};
use flate2::{write::GzEncoder, Compression};

#[derive(Deserialize)]
struct VersionsList {
    versions: Vec<EncodableVersion>,
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

impl crate::util::MockAnonymousUser {
    fn reverse_dependencies(&self, krate_name: &str) -> RevDeps {
        let url = format!("/api/v1/crates/{}/reverse_dependencies", krate_name);
        self.get(&url).good()
    }
}

impl crate::util::MockTokenUser {
    /// Yank the specified version of the specified crate and run all pending background jobs
    fn yank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{}/{}/yank", krate_name, version);
        let response = self.delete(&url);
        self.app().run_pending_background_jobs();
        response
    }

    /// Unyank the specified version of the specified crate and run all pending background jobs
    fn unyank(&self, krate_name: &str, version: &str) -> crate::util::Response<OkBool> {
        let url = format!("/api/v1/crates/{}/{}/unyank", krate_name, version);
        let response = self.put(&url, &[]);
        self.app().run_pending_background_jobs();
        response
    }
}

#[test]
fn index() {
    let (app, anon) = TestApp::init().empty();
    let json = anon.search("");
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);

    let krate = app.db(|conn| {
        let u = new_user("foo").create_or_update(conn).unwrap();
        CrateBuilder::new("fooindex", u.id).expect_build(conn)
    });

    let json = anon.search("");
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.crates[0].name, krate.name);
    assert_eq!(json.crates[0].id, krate.name);
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn index_queries() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let (krate, krate2) = app.db(|conn| {
        let krate = CrateBuilder::new("foo_index_queries", user.id)
            .readme("readme")
            .description("description")
            .keyword("kw1")
            .expect_build(conn);

        let krate2 = CrateBuilder::new("BAR_INDEX_QUERIES", user.id)
            .keyword("KW1")
            .expect_build(conn);

        CrateBuilder::new("foo", user.id)
            .keyword("kw3")
            .expect_build(conn);
        (krate, krate2)
    });

    assert_eq!(anon.search("q=baz").meta.total, 0);

    // All of these fields should be indexed/searched by the queries
    assert_eq!(anon.search("q=foo").meta.total, 2);
    assert_eq!(anon.search("q=kw1").meta.total, 2);
    assert_eq!(anon.search("q=readme").meta.total, 1);
    assert_eq!(anon.search("q=description").meta.total, 1);

    assert_eq!(anon.search_by_user_id(user.id).crates.len(), 3);
    assert_eq!(anon.search_by_user_id(0).crates.len(), 0);

    assert_eq!(anon.search("letter=F").crates.len(), 2);
    assert_eq!(anon.search("letter=B").crates.len(), 1);
    assert_eq!(anon.search("letter=b").crates.len(), 1);
    assert_eq!(anon.search("letter=c").crates.len(), 0);

    assert_eq!(anon.search("keyword=kw1").crates.len(), 2);
    assert_eq!(anon.search("keyword=KW1").crates.len(), 2);
    assert_eq!(anon.search("keyword=kw2").crates.len(), 0);

    assert_eq!(anon.search("q=foo&keyword=kw1").crates.len(), 1);
    assert_eq!(anon.search("q=foo2&keyword=kw1").crates.len(), 0);

    app.db(|conn| {
        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
        new_category("Category 1::Ba'r", "cat1::bar", "Ba'r crates")
            .create_or_update(conn)
            .unwrap();
        Category::update_crate(conn, &krate, &["cat1"]).unwrap();
        Category::update_crate(conn, &krate2, &["cat1::bar"]).unwrap();
    });

    let cl = anon.search("category=cat1");
    assert_eq!(cl.crates.len(), 2);
    assert_eq!(cl.meta.total, 2);

    let cl = anon.search("category=cat1::bar");
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);

    let cl = anon.search("keyword=cat2");
    assert_eq!(cl.crates.len(), 0);
    assert_eq!(cl.meta.total, 0);

    let cl = anon.search("q=readme&category=cat1");
    assert_eq!(cl.crates.len(), 1);
    assert_eq!(cl.meta.total, 1);

    let cl = anon.search("keyword=kw1&category=cat1");
    assert_eq!(cl.crates.len(), 2);
    assert_eq!(cl.meta.total, 2);

    let cl = anon.search("keyword=kw3&category=cat1");
    assert_eq!(cl.crates.len(), 0);
    assert_eq!(cl.meta.total, 0);
}

#[test]
fn search_includes_crates_where_name_is_stopword() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("which", user.id).expect_build(conn);
        CrateBuilder::new("should_be_excluded", user.id)
            .readme("crate which does things")
            .expect_build(conn);
    });
    let json = anon.search("q=which");
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
}

#[test]
fn exact_match_first_on_queries() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_exact", user.id)
            .description("bar_exact baz_exact")
            .expect_build(conn);

        CrateBuilder::new("bar-exact", user.id)
            .description("foo_exact baz_exact foo-exact baz_exact")
            .expect_build(conn);

        CrateBuilder::new("baz_exact", user.id)
            .description("foo-exact bar_exact foo-exact bar_exact foo_exact bar_exact")
            .expect_build(conn);

        CrateBuilder::new("other_exact", user.id)
            .description("other_exact")
            .expect_build(conn);
    });

    let json = anon.search("q=foo-exact");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "foo_exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "bar-exact");

    let json = anon.search("q=bar_exact");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar-exact");
    assert_eq!(json.crates[1].name, "baz_exact");
    assert_eq!(json.crates[2].name, "foo_exact");

    let json = anon.search("q=baz_exact");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_exact");
    assert_eq!(json.crates[1].name, "bar-exact");
    assert_eq!(json.crates[2].name, "foo_exact");
}

#[test]
fn index_sorting() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let krate1 = CrateBuilder::new("foo_sort", user.id)
            .description("bar_sort baz_sort const")
            .downloads(50)
            .recent_downloads(50)
            .expect_build(conn);

        let krate2 = CrateBuilder::new("bar_sort", user.id)
            .description("foo_sort baz_sort foo_sort baz_sort const")
            .downloads(3333)
            .recent_downloads(0)
            .expect_build(conn);

        let krate3 = CrateBuilder::new("baz_sort", user.id)
            .description("foo_sort bar_sort foo_sort bar_sort foo_sort bar_sort const")
            .downloads(100_000)
            .recent_downloads(10)
            .expect_build(conn);

        let krate4 = CrateBuilder::new("other_sort", user.id)
            .description("other_sort const")
            .downloads(999_999)
            .expect_build(conn);

        // Set the updated at column for each crate
        update(&krate1)
            .set(crates::updated_at.eq(now - 3.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate2)
            .set(crates::updated_at.eq(now - 5.days()))
            .execute(conn)
            .unwrap();
        update(&krate3)
            .set(crates::updated_at.eq(now - 10.seconds()))
            .execute(conn)
            .unwrap();
        update(&krate4)
            .set(crates::updated_at.eq(now))
            .execute(conn)
            .unwrap();
    });

    // Sort by downloads
    let json = anon.search("sort=downloads");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "other_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "foo_sort");

    // Sort by recent-downloads
    let json = anon.search("sort=recent-downloads");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "other_sort");

    // Sort by recent-updates
    let json = anon.search("sort=recent-updates");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "other_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "foo_sort");

    // Test for bug with showing null results first when sorting
    // by descending downloads
    let json = anon.search("sort=recent-downloads");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "other_sort");
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn exact_match_on_queries_with_sort() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let krate1 = CrateBuilder::new("foo_sort", user.id)
            .description("bar_sort baz_sort const")
            .downloads(50)
            .recent_downloads(50)
            .expect_build(conn);

        let krate2 = CrateBuilder::new("bar_sort", user.id)
            .description("foo_sort baz_sort foo_sort baz_sort const")
            .downloads(3333)
            .recent_downloads(0)
            .expect_build(conn);

        let krate3 = CrateBuilder::new("baz_sort", user.id)
            .description("foo_sort bar_sort foo_sort bar_sort foo_sort bar_sort const")
            .downloads(100_000)
            .recent_downloads(10)
            .expect_build(conn);

        let krate4 = CrateBuilder::new("other_sort", user.id)
            .description("other_sort const")
            .downloads(999_999)
            .expect_build(conn);

        // Set the updated at column for each crate
        update(&krate1)
            .set(crates::updated_at.eq(now - 3.weeks()))
            .execute(&*conn)
            .unwrap();
        update(&krate2)
            .set(crates::updated_at.eq(now - 5.days()))
            .execute(&*conn)
            .unwrap();
        update(&krate3)
            .set(crates::updated_at.eq(now - 10.seconds()))
            .execute(&*conn)
            .unwrap();
        update(&krate4)
            .set(crates::updated_at.eq(now))
            .execute(&*conn)
            .unwrap();
    });

    // Sort by downloads
    let json = anon.search("q=foo_sort&sort=downloads");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");

    let json = anon.search("q=bar_sort&sort=downloads");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "foo_sort");

    let json = anon.search("q=baz_sort&sort=downloads");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_sort");
    assert_eq!(json.crates[1].name, "bar_sort");
    assert_eq!(json.crates[2].name, "foo_sort");

    let json = anon.search("q=const&sort=downloads");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "other_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "foo_sort");

    // Sort by recent-downloads
    let json = anon.search("q=bar_sort&sort=recent-downloads");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "foo_sort");
    assert_eq!(json.crates[2].name, "baz_sort");

    // Sort by recent-updates
    let json = anon.search("q=bar_sort&sort=recent-updates");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "baz_sort");
    assert_eq!(json.crates[1].name, "bar_sort");
    assert_eq!(json.crates[2].name, "foo_sort");

    // Test for bug with showing null results first when sorting
    // by descending downloads
    let json = anon.search("sort=recent-downloads");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "foo_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "bar_sort");
    assert_eq!(json.crates[3].name, "other_sort");
}

#[test]
fn loose_search_order() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let ordered = app.db(|conn| {
        // exact match should be first
        let one = CrateBuilder::new("temp", user.id)
            .readme("readme")
            .description("description")
            .keyword("kw1")
            .expect_build(conn);
        // temp_udp should match second because of _
        let two = CrateBuilder::new("temp_utp", user.id)
            .readme("readme")
            .description("description")
            .keyword("kw1")
            .expect_build(conn);
        // evalrs should match 3rd because of readme
        let three = CrateBuilder::new("evalrs", user.id)
            .readme("evalrs_temp evalrs_temp evalrs_temp")
            .description("description")
            .keyword("kw1")
            .expect_build(conn);
        // tempfile should appear 4th
        let four = CrateBuilder::new("tempfile", user.id)
            .readme("readme")
            .description("description")
            .keyword("kw1")
            .expect_build(conn);
        vec![one, two, three, four]
    });
    let search_temp = anon.search("q=temp");
    assert_eq!(search_temp.meta.total, 4);
    assert_eq!(search_temp.crates.len(), 4);
    for (lhs, rhs) in search_temp.crates.iter().zip(ordered) {
        assert_eq!(lhs.name, rhs.name);
    }

    let search_temp = anon.search("q=te");
    assert_eq!(search_temp.meta.total, 3);
    assert_eq!(search_temp.crates.len(), 3);
}

#[test]
fn show() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| {
        let krate = CrateBuilder::new("foo_show", user.id)
            .description("description")
            .documentation("https://example.com")
            .homepage("http://example.com")
            .version(VersionBuilder::new("1.0.0"))
            .version(VersionBuilder::new("0.5.0"))
            .version(VersionBuilder::new("0.5.1"))
            .keyword("kw1")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(conn);

        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        krate
    });

    let json = anon.show_crate("foo_show");
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
    assert!(json.versions[0].published_by.is_none());
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
    assert_eq!(
        json.versions[1].published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn yanked_versions_are_not_considered_for_max_version() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_yanked_version", user.id)
            .description("foo")
            .version("1.0.0")
            .version(VersionBuilder::new("1.1.0").yanked(true))
            .expect_build(conn);
    });

    let json = anon.search("q=foo");
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.crates[0].max_version, "1.0.0");
}

#[test]
fn versions() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_versions", user.id)
            .version("0.5.1")
            .version("1.0.0")
            .version("0.5.0")
            .expect_build(conn);
        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();
    });

    let json: VersionsList = anon.get("/api/v1/crates/foo_versions/versions").good();

    assert_eq!(json.versions.len(), 3);
    assert_eq!(json.versions[0].num, "1.0.0");
    assert_eq!(json.versions[1].num, "0.5.1");
    assert_eq!(json.versions[2].num, "0.5.0");
    assert!(json.versions[0].published_by.is_none());
    assert_eq!(
        json.versions[1].published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn uploading_new_version_touches_crate() {
    use diesel::dsl::*;

    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at").version("1.0.0");
    user.enqueue_publish(crate_to_publish).good();

    app.db(|conn| {
        diesel::update(crates::table)
            .set(crates::updated_at.eq(crates::updated_at - 1.hour()))
            .execute(&*conn)
            .unwrap();
    });

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_before = json.krate.updated_at;

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at").version("2.0.0");
    user.enqueue_publish(crate_to_publish).good();

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_after = json.krate.updated_at;

    assert_ne!(updated_at_before, updated_at_after);
}

#[test]
fn new_wrong_token() {
    let (app, anon, _, token) = TestApp::init().with_token();

    // Try to publish without a token
    let crate_to_publish = PublishBuilder::new("foo");
    let json = anon.enqueue_publish(crate_to_publish).bad_with_status(403);
    assert!(
        json.errors[0]
            .detail
            .contains("must be logged in to perform that action"),
        "{:?}",
        json.errors
    );

    // Try to publish with the wrong token (by changing the token in the database)
    app.db(|conn| {
        diesel::update(api_tokens::table)
            .set(api_tokens::token.eq("bad"))
            .execute(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo");
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(403);
    assert!(
        json.errors[0]
            .detail
            .contains("must be logged in to perform that action"),
        "{:?}",
        json.errors
    );
}

#[test]
fn invalid_names() {
    let (_, _, _, token) = TestApp::init().with_token();

    let bad_name = |name: &str, error_message: &str| {
        let crate_to_publish = PublishBuilder::new(name).version("1.0.0");
        let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

        assert!(
            json.errors[0].detail.contains(error_message,),
            "{:?}",
            json.errors
        );
    };

    let error_message = "expected a valid crate name";
    bad_name("", error_message);
    bad_name("foo bar", error_message);
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1), error_message);
    bad_name("snow☃", error_message);
    bad_name("áccênts", error_message);

    let error_message = "cannot upload a crate with a reserved name";
    bad_name("std", error_message);
    bad_name("STD", error_message);
    bad_name("compiler-rt", error_message);
    bad_name("compiler_rt", error_message);
    bad_name("coMpiLer_Rt", error_message);
}

#[test]
fn new_krate() {
    let (_, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_new").version("1.0.0");
    let json: GoodCrate = user.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_with_token() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_new").version("1.0.0");
    let json: GoodCrate = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_weird_version() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_weird").version("0.0.0-pre");
    let json: GoodCrate = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_weird");
    assert_eq!(json.krate.max_version, "0.0.0-pre");
}

#[test]
fn new_with_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("package-name").rename("my-name");

    let crate_to_publish = PublishBuilder::new("new-krate")
        .version("1.0.0")
        .dependency(dependency);
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("ne/w-/new-krate");
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "new-krate");
    assert_eq!(crates[0].vers, "1.0.0");
    assert_eq!(crates[0].deps.len(), 1);
    assert_eq!(crates[0].deps[0].name, "my-name");
    assert_eq!(crates[0].deps[0].package.as_ref().unwrap(), "package-name");
}

#[test]
fn new_krate_with_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_dep can depend on it
        // The name choice of `foo-dep` is important! It has the property of
        // name != canon_crate_name(name) and is a regression test for
        // https://github.com/rust-lang/crates.io/issues/651
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep");

    let crate_to_publish = PublishBuilder::new("new_dep")
        .version("1.0.0")
        .dependency(dependency);
    token.enqueue_publish(crate_to_publish).good();
}

#[test]
fn reject_new_krate_with_non_exact_dependency() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    // Use non-exact name for the dependency
    let dependency = DependencyBuilder::new("foo_dep");

    let crate_to_publish = PublishBuilder::new("new_dep")
        .version("1.0.0")
        .dependency(dependency);
    token.enqueue_publish(crate_to_publish).bad_with_status(200);
}

#[test]
fn new_crate_allow_empty_alternative_registry_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep").registry("");
    let crate_to_publish = PublishBuilder::new("foo").dependency(dependency);
    token.enqueue_publish(crate_to_publish).good();
}

#[test]
fn reject_new_crate_with_alternative_registry_dependency() {
    let (_, _, _, token) = TestApp::init().with_token();

    let dependency =
        DependencyBuilder::new("dep").registry("https://server.example/path/to/registry");

    let crate_to_publish = PublishBuilder::new("depends-on-alt-registry").dependency(dependency);
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        json.errors[0]
            .detail
            .contains("Cross-registry dependencies are not permitted on crates.io."),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_with_wildcard_dependency() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_wild can depend on it
        CrateBuilder::new("foo_wild", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo_wild").version_req("*");

    let crate_to_publish = PublishBuilder::new("new_wild")
        .version("1.0.0")
        .dependency(dependency);

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        json.errors[0].detail.contains("dependency constraints"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_twice() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database and then we'll try to publish another version
        CrateBuilder::new("foo_twice", user.as_model().id).expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_twice")
        .version("2.0.0")
        .description("2.0.0 description");
    let json = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_twice");
    assert_eq!(json.krate.description.unwrap(), "2.0.0 description");
}

#[test]
fn new_krate_wrong_user() {
    let (app, _, user) = TestApp::init().with_user();

    app.db(|conn| {
        // Create the foo_wrong crate with one user
        CrateBuilder::new("foo_wrong", user.as_model().id).expect_build(conn);
    });

    // Then try to publish with a different user
    let another_user = app.db_new_user("another").db_new_token("bar");
    let crate_to_publish = PublishBuilder::new("foo_wrong").version("2.0.0");

    let json = another_user
        .enqueue_publish(crate_to_publish)
        .bad_with_status(200);
    assert!(
        json.errors[0]
            .detail
            .contains("this crate exists but you don't seem to be an owner."),
        "{:?}",
        json.errors
    );
}

// TODO: Move this test to the main crate
#[test]
fn valid_feature_names() {
    assert!(Crate::valid_feature("foo"));
    assert!(!Crate::valid_feature(""));
    assert!(!Crate::valid_feature("/"));
    assert!(!Crate::valid_feature("%/%"));
    assert!(Crate::valid_feature("a/a"));
    assert!(Crate::valid_feature("32-column-tables"));
}

#[test]
fn new_krate_too_big() {
    let (_, _, user) = TestApp::init().with_user();

    let files = [("foo_big-1.0.0/big", &[b'a'; 2000] as &[_])];
    let builder = PublishBuilder::new("foo_big").files(&files);

    let json = user.enqueue_publish(builder).bad_with_status(200);
    assert!(
        json.errors[0]
            .detail
            .contains("uploaded tarball is malformed or too large when decompressed"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_too_big_but_whitelisted() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_whitelist", user.as_model().id)
            .max_upload_size(2_000_000)
            .expect_build(conn);
    });

    let files = [("foo_whitelist-1.1.0/big", &[b'a'; 2000] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_whitelist")
        .version("1.1.0")
        .files(&files);

    token.enqueue_publish(crate_to_publish).good();
}

#[test]
fn new_krate_wrong_files() {
    let (_, _, user) = TestApp::init().with_user();
    let data: &[u8] = &[1];
    let files = [("foo-1.0.0/a", data), ("bar-1.0.0/a", data)];
    let builder = PublishBuilder::new("foo").files(&files);

    let json = user.enqueue_publish(builder).bad_with_status(200);
    assert!(
        json.errors[0].detail.contains("invalid tarball uploaded"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_gzip_bomb() {
    let (_, _, _, token) = TestApp::init().with_token();

    let len = 512 * 1024;
    let mut body = io::repeat(0).take(len);

    let crate_to_publish = PublishBuilder::new("foo")
        .version("1.1.0")
        .files_with_io(&mut [("foo-1.1.0/a", &mut body, len)]);

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0]
            .detail
            .contains("too large when decompressed"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_duplicate_version() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database and then we'll try to publish the same version
        CrateBuilder::new("foo_dupe", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_dupe").version("1.0.0");
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0].detail.contains("already uploaded"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("Foo_similar", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_similar").version("1.1.0");
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name_hyphen() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_bar_hyphen", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo-bar-hyphen").version("1.1.0");
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_crate_similar_name_underscore() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-bar-underscore", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_bar_underscore").version("1.1.0");
    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0].detail.contains("previously named"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_git_upload() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("fgt");
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("3/f/fgt");
    assert!(crates.len() == 1);
    assert_eq!(crates[0].name, "fgt");
    assert_eq!(crates[0].vers, "1.0.0");
    assert!(crates[0].deps.is_empty());
    assert_eq!(
        crates[0].cksum,
        "acb5604b126ac894c1eb11c4575bf2072fea61232a888e453770c79d7ed56419"
    );
}

#[test]
fn new_krate_git_upload_appends() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("FPP").version("0.0.1");
    token.enqueue_publish(crate_to_publish).good();
    let crate_to_publish = PublishBuilder::new("FPP").version("1.0.0");
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("3/f/fpp");
    assert!(crates.len() == 2);
    assert_eq!(crates[0].name, "FPP");
    assert_eq!(crates[0].vers, "0.0.1");
    assert!(crates[0].deps.is_empty());
    assert_eq!(crates[1].name, "FPP");
    assert_eq!(crates[1].vers, "1.0.0");
    assert!(crates[1].deps.is_empty());
}

#[test]
fn new_krate_git_upload_with_conflicts() {
    let (app, _, _, token) = TestApp::full().with_token();

    let index = app.upstream_repository();
    let target = index.head().unwrap().target().unwrap();
    let sig = index.signature().unwrap();
    let parent = index.find_commit(target).unwrap();
    let tree = index.find_tree(parent.tree_id()).unwrap();
    index
        .commit(Some("HEAD"), &sig, &sig, "empty commit", &tree, &[&parent])
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo_conflicts");
    token.enqueue_publish(crate_to_publish).good();
}

#[test]
fn new_krate_dependency_missing() {
    let (_, _, _, token) = TestApp::init().with_token();

    // Deliberately not inserting this crate in the database to test behavior when a dependency
    // doesn't exist!
    let dependency = DependencyBuilder::new("bar_missing");
    let crate_to_publish = PublishBuilder::new("foo_missing").dependency(dependency);

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        json.errors[0]
            .detail
            .contains("no known crate named `bar_missing`"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_with_readme() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme").readme("");
    let json = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn new_krate_without_any_email_fails() {
    let (app, _, _, token) = TestApp::init().with_token();

    app.db(|conn| {
        delete(emails::table).execute(conn).unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_no_email");

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0]
            .detail
            .contains("A verified email address is required to publish crates to crates.io"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_with_unverified_email_fails() {
    let (app, _, _, token) = TestApp::init().with_token();

    app.db(|conn| {
        update(emails::table)
            .set((emails::verified.eq(false),))
            .execute(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_unverified_email");

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0]
            .detail
            .contains("A verified email address is required to publish crates to crates.io"),
        "{:?}",
        json.errors
    );
}

#[test]
fn new_krate_records_verified_email() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_verified_email");

    token.enqueue_publish(crate_to_publish).good();

    app.db(|conn| {
        let email = versions_published_by::table
            .select(versions_published_by::email)
            .first::<String>(conn)
            .unwrap();
        assert_eq!(email, "something@example.com");
    });
}

#[test]
fn summary_doesnt_die() {
    let (_, anon) = TestApp::init().empty();
    anon.get::<SummaryResponse>("/api/v1/summary").good();
}

#[test]
fn summary_new_crates() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        let krate = CrateBuilder::new("some_downloads", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .description("description")
            .keyword("popular")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(conn);

        let krate2 = CrateBuilder::new("most_recent_downloads", user.id)
            .version(VersionBuilder::new("0.2.0"))
            .keyword("popular")
            .downloads(5000)
            .recent_downloads(50)
            .expect_build(conn);

        let krate3 = CrateBuilder::new("just_updated", user.id)
            .version(VersionBuilder::new("0.1.0"))
            .expect_build(conn);

        CrateBuilder::new("with_downloads", user.id)
            .version(VersionBuilder::new("0.3.0"))
            .keyword("popular")
            .downloads(1000)
            .expect_build(conn);

        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
        Category::update_crate(conn, &krate, &["cat1"]).unwrap();
        Category::update_crate(conn, &krate2, &["cat1"]).unwrap();

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
    });

    let json: SummaryResponse = anon.get("/api/v1/summary").good();

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
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_download", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    let assert_dl_count = |name_and_version: &str, query: Option<&str>, count: i32| {
        let url = format!("/api/v1/crates/{}/downloads", name_and_version);
        let downloads: Downloads = if let Some(query) = query {
            anon.get_with_query(&url, query).good()
        } else {
            anon.get(&url).good()
        };
        let total_downloads = downloads
            .version_downloads
            .iter()
            .map(|vd| vd.downloads)
            .sum::<i32>();
        assert_eq!(total_downloads, count);
    };

    let download = |name_and_version: &str| {
        let url = format!("/api/v1/crates/{}/download", name_and_version);
        anon.get::<()>(&url).assert_status(302);
        // TODO: test the with_json code path
    };

    download("foo_download/1.0.0");
    assert_dl_count("foo_download/1.0.0", None, 1);
    assert_dl_count("foo_download", None, 1);

    download("FOO_DOWNLOAD/1.0.0");
    assert_dl_count("FOO_DOWNLOAD/1.0.0", None, 2);
    assert_dl_count("FOO_DOWNLOAD", None, 2);

    let yesterday = (Utc::today() + Duration::days(-1)).format("%F");
    let query = format!("before_date={}", yesterday);
    assert_dl_count("FOO_DOWNLOAD/1.0.0", Some(&query), 0);
    // crate/downloads always returns the last 90 days and ignores date params
    assert_dl_count("FOO_DOWNLOAD", Some(&query), 2);

    let tomorrow = (Utc::today() + Duration::days(1)).format("%F");
    let query = format!("before_date={}", tomorrow);
    assert_dl_count("FOO_DOWNLOAD/1.0.0", Some(&query), 2);
    assert_dl_count("FOO_DOWNLOAD", Some(&query), 2);
}

#[test]
fn download_nonexistent_version_of_existing_crate_404s() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_bad", user.id).expect_build(conn);
    });

    anon.get("/api/v1/crates/foo_bad/0.1.0/download")
        .assert_not_found();
}

#[test]
fn download_noncanonical_crate_name() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_download", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    // Request download for "foo-download" with a dash instead of an underscore,
    // and assert that the correct download link is returned.
    anon.get::<()>("/api/v1/crates/foo-download/1.0.0/download")
        .assert_redirect_ends_with("/crates/foo_download/foo_download-1.0.0.crate");
}

#[test]
fn dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("foo_deps", user.id).expect_build(conn);
        let v = VersionBuilder::new("1.0.0").expect_build(c1.id, user.id, conn);
        let c2 = CrateBuilder::new("bar_deps", user.id).expect_build(conn);
        new_dependency(conn, &v, &c2);
    });

    let deps: Deps = anon
        .get("/api/v1/crates/foo_deps/1.0.0/dependencies")
        .good();
    assert_eq!(deps.dependencies[0].crate_id, "bar_deps");

    anon.get::<()>("/api/v1/crates/foo_deps/1.0.2/dependencies")
        .bad_with_status(200);
}

#[test]
fn diesel_not_found_results_in_404() {
    let (_, _, user) = TestApp::init().with_user();

    user.get("/api/v1/crates/foo_following/following")
        .assert_not_found();
}

#[test]
fn following() {
    // TODO: Test anon requests as well?
    let (app, _, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("foo_following", user.as_model().id).expect_build(conn);
    });

    let is_following = || -> bool {
        #[derive(Deserialize)]
        struct F {
            following: bool,
        }

        user.get::<F>("/api/v1/crates/foo_following/following")
            .good()
            .following
    };

    let follow = || {
        assert!(
            user.put::<OkBool>("/api/v1/crates/foo_following/follow", b"")
                .good()
                .ok
        );
    };

    let unfollow = || {
        assert!(
            user.delete::<OkBool>("api/v1/crates/foo_following/follow")
                .good()
                .ok
        );
    };

    assert!(!is_following());
    follow();
    follow();
    assert!(is_following());
    assert_eq!(user.search("following=1").crates.len(), 1);

    unfollow();
    unfollow();
    assert!(!is_following());
    assert_eq!(user.search("following=1").crates.len(), 0);
}

#[test]
fn yank_works_as_intended() {
    let (app, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert!(crates.len() == 1);
    assert!(!crates[0].yanked.unwrap());

    // make sure it's not yanked
    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    token.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert!(crates.len() == 1);
    assert!(crates[0].yanked.unwrap());

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    token.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert!(crates.len() == 1);
    assert!(!crates[0].yanked.unwrap());

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);
}

#[test]
fn yank_by_a_non_owner_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    let another_user = app.db_new_user("bar");
    let another_user = another_user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_not", another_user.id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let json = token.yank("foo_not", "1.0.0").bad_with_status(200);
    assert_eq!(
        json.errors[0].detail,
        "must already be an owner to yank or unyank"
    );
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.enqueue_publish(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    token.yank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    token.unyank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.enqueue_publish(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_removing_documentation() {
    let (app, anon, user, token) = TestApp::full().with_token();
    let user = user.as_model();

    // 1. Start with a crate with no documentation
    app.db(|conn| {
        CrateBuilder::new("docscrate", user.id)
            .version("0.2.0")
            .expect_build(conn);
    });

    // Verify that crates start without any documentation so the next assertion can *prove*
    // that it was the one that added the documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, None);

    // 2. Add documentation
    let crate_to_publish = PublishBuilder::new("docscrate")
        .version("0.2.1")
        .documentation("http://foo.rs");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // Ensure latest version also has the same documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // 3. Remove the documentation
    let crate_to_publish = PublishBuilder::new("docscrate").version("0.2.2");
    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.documentation, None);

    // Ensure latest version no longer has documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, None);
}

#[test]
fn bad_keywords() {
    let (_, _, _, token) = TestApp::init().with_token();
    let crate_to_publish =
        PublishBuilder::new("foo_bad_key").keyword("super-long-keyword-name-oh-no");
    token.enqueue_publish(crate_to_publish).bad_with_status(200);

    let crate_to_publish = PublishBuilder::new("foo_bad_key").keyword("?@?%");
    token.enqueue_publish(crate_to_publish).bad_with_status(200);

    let crate_to_publish = PublishBuilder::new("foo_bad_key").keyword("áccênts");
    token.enqueue_publish(crate_to_publish).bad_with_status(200);
}

#[test]
fn good_categories() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_good_cat").category("cat1");
    let json = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_good_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories.len(), 0);
}

#[test]
fn ignored_categories() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_ignored_cat").category("bar");
    let json = token.enqueue_publish(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_ignored_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories, vec!["bar"]);
}

#[test]
fn good_badges() {
    let (_, anon, _, token) = TestApp::full().with_token();

    let mut badges = HashMap::new();
    let mut badge_attributes = HashMap::new();
    badge_attributes.insert(
        String::from("repository"),
        String::from("rust-lang/crates.io"),
    );
    badges.insert(String::from("travis-ci"), badge_attributes);

    let crate_to_publish = PublishBuilder::new("foobadger").badges(badges);

    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.name, "foobadger");
    assert_eq!(json.krate.max_version, "1.0.0");

    let json = anon.show_crate("foobadger");
    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0].badge_type, "travis-ci");
    assert_eq!(
        badges[0].attributes["repository"],
        Some(String::from("rust-lang/crates.io"))
    );
}

#[test]
fn ignored_badges() {
    let (_, anon, _, token) = TestApp::full().with_token();

    let mut badges = HashMap::new();

    // Known badge type, missing required repository attribute
    let mut badge_attributes = HashMap::new();
    badge_attributes.insert(String::from("branch"), String::from("master"));
    badges.insert(String::from("travis-ci"), badge_attributes);

    // Unknown badge type
    let mut unknown_badge_attributes = HashMap::new();
    unknown_badge_attributes.insert(String::from("repository"), String::from("rust-lang/rust"));
    badges.insert(String::from("not-a-badge"), unknown_badge_attributes);

    let crate_to_publish = PublishBuilder::new("foo_ignored_badge").badges(badges);

    let json = token.enqueue_publish(crate_to_publish).good();
    assert_eq!(json.krate.name, "foo_ignored_badge");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_badges.len(), 2);
    assert!(json
        .warnings
        .invalid_badges
        .contains(&"travis-ci".to_string(),));
    assert!(json
        .warnings
        .invalid_badges
        .contains(&"not-a-badge".to_string(),));

    let json = anon.show_crate("foo_ignored_badge");
    let badges = json.krate.badges.unwrap();
    assert_eq!(badges.len(), 0);
}

#[test]
fn reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id).expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version(
                VersionBuilder::new("1.1.0")
                    .dependency(&c1, None)
                    .dependency(&c1, Some("foo")),
            )
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
    assert_eq!(deps.versions.len(), 1);
    assert_eq!(deps.versions[0].krate, "c2");
    assert_eq!(deps.versions[0].num, "1.1.0");

    // c1 has no dependent crates.
    let deps = anon.reverse_dependencies("c2");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn reverse_dependencies_when_old_version_doesnt_depend_but_new_does() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.1.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.0.0")
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn reverse_dependencies_when_old_version_depended_but_new_doesnt() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("2.0.0")
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn prerelease_versions_not_included_in_reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.1.0-pre")
            .expect_build(conn);
        CrateBuilder::new("c3", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("1.1.0-pre")
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn yanked_versions_not_included_in_reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.0.0")
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");

    app.db(|conn| {
        diesel::update(versions::table.filter(versions::num.eq("2.0.0")))
            .set(versions::yanked.eq(true))
            .execute(conn)
            .unwrap();
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn reverse_dependencies_includes_published_by_user_when_present() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);

        // Make c2's version (and,incidentally, c1's, but that doesn't matter) mimic a version
        // published before we started recording who published versions
        let none: Option<i32> = None;
        update(versions::table)
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        // c3's version will have the published by info recorded
        CrateBuilder::new("c3", user.id)
            .version(VersionBuilder::new("3.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.versions.len(), 2);

    let c2_version = deps.versions.iter().find(|v| v.krate == "c2").unwrap();
    assert!(c2_version.published_by.is_none());

    let c3_version = deps.versions.iter().find(|v| v.krate == "c3").unwrap();
    assert_eq!(
        c3_version.published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn author_license_and_description_required() {
    let (_, _, _, token) = TestApp::init().with_token();

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_license()
        .unset_description()
        .unset_authors();

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        json.errors[0].detail.contains("author")
            && json.errors[0].detail.contains("description")
            && json.errors[0].detail.contains("license"),
        "{:?}",
        json.errors
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_description()
        .unset_authors()
        .author("");

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        json.errors[0].detail.contains("author")
            && json.errors[0].detail.contains("description")
            && !json.errors[0].detail.contains("license"),
        "{:?}",
        json.errors
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_license()
        .license_file("foo")
        .unset_description();

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);
    assert!(
        !json.errors[0].detail.contains("author")
            && json.errors[0].detail.contains("description")
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
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        // More than 90 days ago
        CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .downloads(10)
            .recent_downloads(0)
            .expect_build(conn);

        CrateBuilder::new("sweet_potato_snack", user.id)
            .description("For when better than usual")
            .downloads(5)
            .recent_downloads(2)
            .expect_build(conn);
    });

    let json = anon.search("sort=recent-downloads");

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
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        // More than 90 days ago
        CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .downloads(0)
            .recent_downloads(0)
            .expect_build(conn);
    });

    let json = anon.search("sort=recent-downloads");
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
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let (green_crate, potato_crate) = app.db(|conn| {
        // More than 90 days ago
        let green_crate = CrateBuilder::new("green_ball", user.id)
            .description("For fetching")
            .keyword("dog")
            .downloads(10)
            .recent_downloads(10)
            .expect_build(conn);

        let potato_crate = CrateBuilder::new("sweet_potato_snack", user.id)
            .description("For when better than usual")
            .keyword("dog")
            .downloads(20)
            .recent_downloads(0)
            .expect_build(conn);

        (green_crate, potato_crate)
    });

    // test that index for keywords is sorted by recent_downloads
    // by default
    let json = anon.search("keyword=dog");

    assert_eq!(json.meta.total, 2);

    assert_eq!(json.crates[0].name, "green_ball");
    assert_eq!(json.crates[1].name, "sweet_potato_snack");

    assert_eq!(json.crates[0].recent_downloads, Some(10));
    assert_eq!(json.crates[0].downloads, 10);

    assert_eq!(json.crates[1].recent_downloads, Some(0));
    assert_eq!(json.crates[1].downloads, 20);

    app.db(|conn| {
        new_category("Animal", "animal", "animal crates")
            .create_or_update(conn)
            .unwrap();
        Category::update_crate(conn, &green_crate, &["animal"]).unwrap();
        Category::update_crate(conn, &potato_crate, &["animal"]).unwrap();
    });

    // test that index for categories is sorted by recent_downloads
    // by default
    let json = anon.search("category=animal");

    assert_eq!(json.meta.total, 2);

    assert_eq!(json.crates[0].name, "green_ball");
    assert_eq!(json.crates[1].name, "sweet_potato_snack");

    assert_eq!(json.crates[0].recent_downloads, Some(10));
    assert_eq!(json.crates[0].downloads, 10);

    assert_eq!(json.crates[1].recent_downloads, Some(0));
    assert_eq!(json.crates[1].downloads, 20);
}

#[test]
fn block_bad_documentation_url() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_bad_doc_url", user.id)
            .documentation("http://rust-ci.org/foo/foo_bad_doc_url/doc/foo_bad_doc_url/")
            .expect_build(conn)
    });

    let json = anon.show_crate("foo_bad_doc_url");
    assert_eq!(json.krate.documentation, None);
}

// This is testing Cargo functionality! ! !
// specifically functions modify_owners and add_owners
// which call the `PUT /crates/:crate_id/owners` route
#[test]
fn test_cargo_invite_owners() {
    let (app, _, owner) = TestApp::init().with_user();

    let new_user = app.db_new_user("cilantro");
    app.db(|conn| {
        CrateBuilder::new("guacamole", owner.as_model().id).expect_build(conn);
    });

    #[derive(Serialize)]
    struct OwnerReq {
        owners: Option<Vec<String>>,
    }
    #[derive(Deserialize, Debug)]
    struct OwnerResp {
        // server must include `ok: true` to support old cargo clients
        ok: bool,
        msg: String,
    }

    let body = serde_json::to_string(&OwnerReq {
        owners: Some(vec![new_user.as_model().gh_login.clone()]),
    });
    let json: OwnerResp = owner
        .put("/api/v1/crates/guacamole/owners", body.unwrap().as_bytes())
        .good();

    // this ok:true field is what old versions of Cargo
    // need - do not remove unless you're cool with
    // dropping support for old versions
    assert!(json.ok);
    // msg field is what is sent and used in updated
    // version of cargo
    assert_eq!(
        json.msg,
        "user cilantro has been invited to be an owner of crate guacamole"
    )
}

#[test]
fn new_krate_tarball_with_hard_links() {
    let (_, _, _, token) = TestApp::init().with_token();

    let mut tarball = Vec::new();
    {
        let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
        let mut header = tar::Header::new_gnu();
        t!(header.set_path("foo-1.1.0/bar"));
        header.set_size(0);
        header.set_cksum();
        header.set_entry_type(tar::EntryType::hard_link());
        t!(header.set_link_name("foo-1.1.0/another"));
        t!(ar.append(&header, &[][..]));
        t!(ar.finish());
    }

    let crate_to_publish = PublishBuilder::new("foo").version("1.1.0").tarball(tarball);

    let json = token.enqueue_publish(crate_to_publish).bad_with_status(200);

    assert!(
        json.errors[0]
            .detail
            .contains("too large when decompressed"),
        "{:?}",
        json.errors
    );
}

#[test]
fn publish_new_crate_rate_limited() {
    let (app, anon, _, token) = TestApp::full()
        .with_publish_rate_limit(Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1");
    token.enqueue_publish(crate_to_publish).good();

    // Uploading a second crate is limited
    let crate_to_publish = PublishBuilder::new("rate_limited2");
    token.enqueue_publish(crate_to_publish).assert_status(429);
    app.run_pending_background_jobs();

    anon.get::<()>("/api/v1/crates/rate_limited2")
        .assert_status(404);

    // Wait for the limit to be up
    thread::sleep(Duration::from_millis(500));

    let crate_to_publish = PublishBuilder::new("rate_limited2");
    token.enqueue_publish(crate_to_publish).good();

    let json = anon.show_crate("rate_limited2");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn publish_rate_limit_doesnt_affect_existing_crates() {
    let (app, _, _, token) = TestApp::full()
        .with_publish_rate_limit(Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1");
    token.enqueue_publish(crate_to_publish).good();

    let new_version = PublishBuilder::new("rate_limited1").version("1.0.1");
    token.enqueue_publish(new_version).good();
    app.run_pending_background_jobs();
}
