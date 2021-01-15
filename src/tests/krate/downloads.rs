use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use cargo_registry::views::EncodableVersionDownload;
use chrono::{Duration, Utc};
use http::StatusCode;

#[derive(Deserialize)]
struct Downloads {
    version_downloads: Vec<EncodableVersionDownload>,
}

#[test]
fn download() {
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
        let response = anon.get::<()>(&url);
        assert_eq!(response.status(), StatusCode::FOUND);
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
