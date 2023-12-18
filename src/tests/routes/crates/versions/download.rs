use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

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
fn rejected_non_canonical_download() {
    let (app, anon, user) = TestApp::init().with_user();

    app.db(|conn| {
        let user = user.as_model();
        CrateBuilder::new("foo_download", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    // Request download for "foo-download" with a dash instead of an underscore,
    // and assert that the correct download link is returned.
    let response = anon.get::<()>("/api/v1/crates/foo-download/1.0.0/download");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn force_unconditional_redirect() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.force_unconditional_redirects = true;
        })
        .with_user();

    app.db(|conn| {
        CrateBuilder::new("foo-download", user.as_model().id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    // Any redirect to an existing crate and version works correctly.
    anon.get::<()>("/api/v1/crates/foo-download/1.0.0/download")
        .assert_redirect_ends_with("/crates/foo-download/foo-download-1.0.0.crate");

    // Redirects to crates with wrong capitalization are performed unconditionally.
    anon.get::<()>("/api/v1/crates/Foo_downloaD/1.0.0/download")
        .assert_redirect_ends_with("/crates/Foo_downloaD/Foo_downloaD-1.0.0.crate");

    // Redirects to missing versions are performed unconditionally.
    anon.get::<()>("/api/v1/crates/foo-download/2.0.0/download")
        .assert_redirect_ends_with("/crates/foo-download/foo-download-2.0.0.crate");

    // Redirects to missing crates are performed unconditionally.
    anon.get::<()>("/api/v1/crates/bar-download/1.0.0/download")
        .assert_redirect_ends_with("/crates/bar-download/bar-download-1.0.0.crate");
}

#[test]
fn download_caches_version_id() {
    use super::super::downloads;
    use crates_io::schema::crates;
    use diesel::prelude::*;

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_download", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    anon.get::<()>("/api/v1/crates/foo_download/1.0.0/download")
        .assert_redirect_ends_with("/crates/foo_download/foo_download-1.0.0.crate");

    // Rename the crate, so that `foo_download` will not be found if its version_id was not cached
    app.db(|conn| {
        diesel::update(crates::table.filter(crates::name.eq("foo_download")))
            .set(crates::name.eq("other"))
            .execute(conn)
            .unwrap();
    });

    // This would result in a 404 if the endpoint tried to read from the database
    anon.get::<()>("/api/v1/crates/foo_download/1.0.0/download")
        .assert_redirect_ends_with("/crates/foo_download/foo_download-1.0.0.crate");

    // Downloads are persisted by version_id, so the rename doesn't matter
    downloads::persist_downloads_count(&app);
    // Check download count against the new name, rather than rename it back to the original value
    downloads::assert_dl_count(&anon, "other/1.0.0", None, 2);
}

#[test]
fn download_with_build_metadata() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo", user.id)
            .version(VersionBuilder::new("1.0.0+bar"))
            .expect_build(conn);
    });

    anon.get::<()>("/api/v1/crates/foo/1.0.0+bar/download")
        .assert_redirect_ends_with("/crates/foo/foo-1.0.0%2Bbar.crate");

    anon.get::<()>("/api/v1/crates/foo/1.0.0+bar/readme")
        .assert_redirect_ends_with("/readmes/foo/foo-1.0.0%2Bbar.html");
}
