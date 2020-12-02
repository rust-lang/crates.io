use crate::{
    builders::{CrateBuilder, PublishBuilder, VersionBuilder},
    new_category, new_user, CrateMeta, OkBool, RequestHelper, TestApp,
};
use cargo_registry::{
    models::Category,
    schema::crates,
    views::{EncodableDependency, EncodableVersion},
};

use conduit::StatusCode;
use diesel::{dsl::*, prelude::*, update};

mod dependencies;
mod downloads;
mod following;
mod publish;
mod summary;
mod versions;

#[derive(Deserialize)]
struct RevDeps {
    dependencies: Vec<EncodableDependency>,
    versions: Vec<EncodableVersion>,
    meta: CrateMeta,
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

impl crate::util::MockCookieUser {
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
        let u = new_user("foo").create_or_update(None, conn).unwrap();
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

        CrateBuilder::new("two-keywords", user.id)
            .keyword("kw1")
            .keyword("kw3")
            .expect_build(conn);
        (krate, krate2)
    });

    assert_eq!(anon.search("q=baz").meta.total, 0);

    // All of these fields should be indexed/searched by the queries
    assert_eq!(anon.search("q=foo").meta.total, 2);
    assert_eq!(anon.search("q=kw1").meta.total, 3);
    assert_eq!(anon.search("q=readme").meta.total, 1);
    assert_eq!(anon.search("q=description").meta.total, 1);

    assert_eq!(anon.search_by_user_id(user.id).crates.len(), 4);
    assert_eq!(anon.search_by_user_id(0).crates.len(), 0);

    assert_eq!(anon.search("letter=F").crates.len(), 2);
    assert_eq!(anon.search("letter=B").crates.len(), 1);
    assert_eq!(anon.search("letter=b").crates.len(), 1);
    assert_eq!(anon.search("letter=c").crates.len(), 0);

    assert_eq!(anon.search("keyword=kw1").crates.len(), 3);
    assert_eq!(anon.search("keyword=KW1").crates.len(), 3);
    assert_eq!(anon.search("keyword=kw2").crates.len(), 0);
    assert_eq!(anon.search("all_keywords=kw1 kw3").crates.len(), 1);

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
#[allow(clippy::cognitive_complexity)]
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

        // Set the created at column for each crate
        update(&krate1)
            .set(crates::created_at.eq(now - 4.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate2)
            .set(crates::created_at.eq(now - 1.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate3)
            .set(crates::created_at.eq(now - 2.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate4)
            .set(crates::created_at.eq(now - 3.weeks()))
            .execute(conn)
            .unwrap();

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

    // Sort by new
    let json = anon.search("sort=new");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
    assert_eq!(json.crates[2].name, "other_sort");
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

        // Set the created at column for each crate
        update(&krate1)
            .set(crates::created_at.eq(now - 4.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate2)
            .set(crates::created_at.eq(now - 1.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate3)
            .set(crates::created_at.eq(now - 2.weeks()))
            .execute(conn)
            .unwrap();
        update(&krate4)
            .set(crates::created_at.eq(now - 3.weeks()))
            .execute(conn)
            .unwrap();

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

    // Sort by new
    let json = anon.search("q=bar_sort&sort=new");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "bar_sort");
    assert_eq!(json.crates[1].name, "baz_sort");
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
fn index_include_yanked() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("unyanked", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .version(VersionBuilder::new("2.0.0"))
            .expect_build(conn);

        CrateBuilder::new("newest_yanked", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .version(VersionBuilder::new("2.0.0").yanked(true))
            .expect_build(conn);

        CrateBuilder::new("oldest_yanked", user.id)
            .version(VersionBuilder::new("1.0.0").yanked(true))
            .version(VersionBuilder::new("2.0.0"))
            .expect_build(conn);

        CrateBuilder::new("all_yanked", user.id)
            .version(VersionBuilder::new("1.0.0").yanked(true))
            .version(VersionBuilder::new("2.0.0").yanked(true))
            .expect_build(conn);
    });

    // Include fully yanked (all versions were yanked) crates
    let json = anon.search("include_yanked=yes&sort=alphabetical");
    assert_eq!(json.meta.total, 4);
    assert_eq!(json.crates[0].name, "all_yanked");
    assert_eq!(json.crates[1].name, "newest_yanked");
    assert_eq!(json.crates[2].name, "oldest_yanked");
    assert_eq!(json.crates[3].name, "unyanked");

    // Do not include fully yanked (all versions were yanked) crates
    let json = anon.search("include_yanked=no&sort=alphabetical");
    assert_eq!(json.meta.total, 3);
    assert_eq!(json.crates[0].name, "newest_yanked");
    assert_eq!(json.crates[1].name, "oldest_yanked");
    assert_eq!(json.crates[2].name, "unyanked");
}

#[test]
fn show() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| {
        use cargo_registry::schema::versions;

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
    assert_none!(&json.versions[0].published_by);
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
fn yank_works_as_intended() {
    let (app, anon, cookie, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();
    app.run_pending_background_jobs();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    // make sure it's not yanked
    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    token.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    token.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    cookie.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    cookie.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("3/f/fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

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

    let json = token
        .yank("foo_not", "1.0.0")
        .bad_with_status(StatusCode::OK);
    assert_eq!(
        json.errors[0].detail,
        "must already be an owner to yank or unyank"
    );
}

#[test]
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
fn yank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();

    // Yank it
    token.yank("fyk", "1.0.0").good();

    // Make sure it has one publish and one yank audit action
    let json = anon.show_version("fyk", "1.0.0");
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 2);
    let action = &actions[1];
    assert_eq!(action.action, "yank");
    assert_eq!(action.user.id, token.as_model().user_id);
}

#[test]
fn unyank_records_an_audit_action() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk");
    token.enqueue_publish(crate_to_publish).good();

    // Yank version 1.0.0
    token.yank("fyk", "1.0.0").good();

    // Unyank version 1.0.0
    token.unyank("fyk", "1.0.0").good();

    // Make sure it has one publish, one yank, and one unyank audit action
    let json = anon.show_version("fyk", "1.0.0");
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 3);
    let action = &actions[2];
    assert_eq!(action.action, "unyank");
    assert_eq!(action.user.id, token.as_model().user_id);
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
        use cargo_registry::schema::versions;

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
        use cargo_registry::schema::versions;

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
    assert_none!(&c2_version.published_by);

    let c3_version = deps.versions.iter().find(|v| v.krate == "c3").unwrap();
    assert_eq!(
        c3_version.published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn reverse_dependencies_query_supports_u64_version_number_parts() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let large_but_valid_version_number = format!("1.0.{}", std::u64::MAX);

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id).expect_build(conn);
        // The crate that depends on c1...
        CrateBuilder::new("c2", user.id)
            // ...has a patch version at the limits of what the semver crate supports
            .version(VersionBuilder::new(&large_but_valid_version_number).dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
    assert_eq!(deps.versions.len(), 1);
    assert_eq!(deps.versions[0].krate, "c2");
    assert_eq!(deps.versions[0].num, large_but_valid_version_number);
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
fn pagination_links_included_if_applicable() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("pagination_links_1", user.id).expect_build(conn);
        CrateBuilder::new("pagination_links_2", user.id).expect_build(conn);
        CrateBuilder::new("pagination_links_3", user.id).expect_build(conn);
    });

    let page1 = anon.search("per_page=1");
    let page2 = anon.search("page=2&per_page=1");
    let page3 = anon.search("page=3&per_page=1");
    let page4 = anon.search("page=4&per_page=1");

    assert_eq!(Some("?per_page=1&page=2".to_string()), page1.meta.next_page);
    assert_eq!(None, page1.meta.prev_page);
    assert_eq!(Some("?page=3&per_page=1".to_string()), page2.meta.next_page);
    assert_eq!(Some("?page=1&per_page=1".to_string()), page2.meta.prev_page);
    assert_eq!(None, page4.meta.next_page);
    assert_eq!(Some("?page=2&per_page=1".to_string()), page3.meta.prev_page);
}

#[test]
fn pagination_parameters_only_accept_integers() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("pagination_links_1", user.id).expect_build(conn);
        CrateBuilder::new("pagination_links_2", user.id).expect_build(conn);
        CrateBuilder::new("pagination_links_3", user.id).expect_build(conn);
    });

    let invalid_per_page_json = anon
        .get_with_query::<()>("/api/v1/crates", "page=1&per_page=100%22%EF%BC%8Cexception")
        .bad_with_status(StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_per_page_json.errors[0].detail,
        "invalid digit found in string"
    );

    let invalid_page_json = anon
        .get_with_query::<()>("/api/v1/crates", "page=100%22%EF%BC%8Cexception&per_page=1")
        .bad_with_status(StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_page_json.errors[0].detail,
        "invalid digit found in string"
    );
}
